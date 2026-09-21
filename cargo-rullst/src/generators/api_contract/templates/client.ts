type Parameter = { name: string; location: string; required: boolean; schema: JsonObject };
type Operation = { id: string; method: string; path: string; parameters: Parameter[]; body: string | null; responses: Record<string, string> };
const MAX_WIRE_BYTES = 64 * 1024;
export type Authorization = () => string | null | Promise<string | null>;

export class ContractClient {
    private readonly origin: string;
    constructor(baseUrl: string, private readonly authorization: Authorization = () => null) {
        try {
            const url = new URL(baseUrl);
            if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash || url.pathname !== '/') reject();
            this.origin = url.origin;
        } catch { throw new ContractError(); }
    }
    private async execute(operation: Operation, input: JsonObject, signal?: AbortSignal): Promise<unknown> {
        let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
        const timeout = new AbortController();
        const timer = setTimeout(() => timeout.abort(), 10_000);
        try {
            const abort = signal ? AbortSignal.any([signal, timeout.signal]) : timeout.signal;
            if (!object(input)) reject();
            const properties: Record<string, JsonObject> = Object.create(null) as Record<string, JsonObject>;
            const required: string[] = [];
            for (const parameter of operation.parameters) {
                const key = (parameter.location === 'path' ? 'p_' : 'q_') + parameter.name;
                properties[key] = parameter.schema;
                if (parameter.required) required.push(key);
            }
            if (operation.body) {
                properties.body = { $ref: '#/components/schemas/' + operation.body };
                required.push('body');
            }
            validate(input, { type: 'object', properties, required });
            let path = operation.path;
            const query = new URLSearchParams();
            for (const parameter of operation.parameters) {
                const key = (parameter.location === 'path' ? 'p_' : 'q_') + parameter.name;
                if (!own(input, key)) continue;
                const value = String(input[key]);
                if (parameter.location === 'path') {
                    if (value === '.' || value === '..' || value.length === 0) reject();
                    path = path.replace('{' + parameter.name + '}', encodeURIComponent(value));
                } else { query.append(parameter.name, value); }
            }
            let body: string | undefined;
            if (operation.body) {
                body = JSON.stringify(input.body);
                if (new TextEncoder().encode(body).length > MAX_WIRE_BYTES) reject();
                validate(strictJson(body), schemas[operation.body]);
            }
            // Timeout includes credential lookup. Caller-controlled providers
            // must also honor their own cancellation/resource lifecycle.
            const token = await new Promise<string | null>((resolve, rejectPromise) => {
                if (abort.aborted) { rejectPromise(new ContractError()); return; }
                const cancel = () => rejectPromise(new ContractError());
                abort.addEventListener('abort', cancel, { once: true });
                Promise.resolve().then(this.authorization).then(resolve, rejectPromise)
                    .finally(() => abort.removeEventListener('abort', cancel));
            });
            const headers: Record<string, string> = { Accept: 'application/json' };
            if (body !== undefined) headers['Content-Type'] = 'application/json';
            if (token !== null) {
                if (typeof token !== 'string' || token.length > 4096 || !/^[A-Za-z0-9._~+\/-]+=*$/.test(token)) reject();
                headers.Authorization = 'Bearer ' + token;
            }
            const suffix = query.toString();
            const response = await fetch(this.origin + path + (suffix ? '?' + suffix : ''), {
                method: operation.method, headers, ...(body === undefined ? {} : {body}), signal: abort,
                redirect: 'error', credentials: 'omit', cache: 'no-store', referrerPolicy: 'no-referrer',
            });
            const component = operation.responses[String(response.status)];
            if (!component || !response.body || response.headers.get('content-type')?.split(';')[0].trim().toLowerCase() !== 'application/json') {
                await response.body?.cancel();
                reject();
            }
            const length = response.headers.get('content-length');
            if (length !== null && (!/^[0-9]+$/.test(length) || Number(length) > MAX_WIRE_BYTES)) {
                await response.body.cancel(); reject();
            }
            reader = response.body.getReader();
            const chunks: Uint8Array[] = [];
            let size = 0;
            while (true) {
                const next = await reader.read();
                if (next.done) break;
                size += next.value.length;
                if (size > MAX_WIRE_BYTES) reject();
                chunks.push(next.value);
            }
            const bytes = new Uint8Array(size);
            let offset = 0;
            for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
            const parsed = strictJson(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
            validate(parsed, schemas[component]);
            return { status: response.status, body: parsed };
        } catch { throw new ContractError(); }
        finally {
            clearTimeout(timer);
            if (reader) { try { await reader.cancel(); } catch { /* Cancelled/failed stream. */ } }
        }
    }
/* GENERATED_METHODS */
}
