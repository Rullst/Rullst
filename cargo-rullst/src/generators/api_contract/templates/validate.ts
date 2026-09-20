type JsonObject = { [key: string]: unknown };
function object(value: unknown): value is JsonObject {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}
function own(value: object, key: string): boolean {
    return Object.prototype.hasOwnProperty.call(value, key);
}
export class ContractError extends Error {
    constructor() { super("API contract or transport rejected"); this.name = "ContractError"; }
}
function reject(): never { throw new ContractError(); }
function validate(value: unknown, schema: JsonObject, depth = 0): void {
    if (depth > 24) reject();
    if (typeof schema.$ref === "string") {
        const name = schema.$ref.slice("#/components/schemas/".length);
        const target = schemas[name];
        if (!target) reject();
        return validate(value, target, depth + 1);
    }
    let type = schema.type;
    if (Array.isArray(type)) {
        if (value === null && type.includes("null")) return;
        type = type.find(t => t !== "null");
    }
    switch (type) {
        case "object": {
            if (!object(value) || (Object.getPrototypeOf(value) !== Object.prototype && Object.getPrototypeOf(value) !== null)) reject();
            const properties = schema.properties as Record<string, JsonObject>;
            for (const key of Object.keys(value)) {
                if (!own(properties, key)) reject();
                const descriptor = Object.getOwnPropertyDescriptor(value, key);
                if (!descriptor || !own(descriptor, "value")) reject();
                validate(descriptor.value, properties[key], depth + 1);
            }
            for (const key of schema.required as string[]) if (!own(value, key)) reject();
            return;
        }
        case "string": {
            if (typeof value !== "string") reject();
            const points = Array.from(value);
            if (points.length < (schema.minLength as number ?? 0) || points.length > (schema.maxLength as number)) reject();
            // Rust strings cannot represent isolated UTF-16 surrogates.
            if (points.some(p => { const n = p.codePointAt(0)!; return n >= 0xd800 && n <= 0xdfff; })) reject();
            return;
        }
        case "integer":
            if (typeof value !== "number" || !Number.isSafeInteger(value) || value < (schema.minimum as number) || value > (schema.maximum as number)) reject();
            return;
        case "boolean": if (typeof value !== "boolean") reject(); return;
        case "array":
            if (!Array.isArray(value) || value.length < (schema.minItems as number ?? 0) || value.length > (schema.maxItems as number)) reject();
            for (let i = 0; i < value.length; i++) {
                const descriptor = Object.getOwnPropertyDescriptor(value, String(i));
                if (!descriptor || !own(descriptor, "value")) reject();
                validate(descriptor.value, schema.items as JsonObject, depth + 1);
            }
            return;
        default: reject();
    }
}
// JSON.parse alone silently accepts duplicate keys. This bounded parser rejects
// them before decoding a server response; object values have no inherited fields.
function strictJson(text: string): unknown {
    let offset = 0, nodes = 0;
    function space(): void { while (/[\x20\t\r\n]/.test(text[offset] ?? "x")) offset++; }
    function string(): string {
        const match = /^"(?:[^"\\\u0000-\u001f]|\\(?:["\\/bfnrt]|u[0-9a-fA-F]{4}))*"/.exec(text.slice(offset));
        if (!match) reject();
        offset += match[0].length;
        return JSON.parse(match[0]) as string;
    }
    function value(depth: number): unknown {
        space();
        if (++nodes > 8192 || depth > 24) reject();
        const token = text[offset];
        if (token === '"') return string();
        if (token === "{" || token === "[") {
            offset++;
            const map: JsonObject = Object.create(null) as JsonObject;
            const list: unknown[] = [];
            const end = token === "{" ? "}" : "]";
            space();
            if (text[offset] === end) { offset++; return token === "{" ? map : list; }
            while (true) {
                space();
                if (token === "{") {
                    const key = string();
                    if (own(map, key)) reject();
                    space();
                    if (text[offset++] !== ":") reject();
                    map[key] = value(depth + 1);
                } else { list.push(value(depth + 1)); }
                space();
                const next = text[offset++];
                if (next === end) return token === "{" ? map : list;
                if (next !== ",") reject();
            }
        }
        const match = /^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/.exec(text.slice(offset));
        if (!match) reject();
        offset += match[0].length;
        const parsed: unknown = /^[-0-9]/.test(match[0]) ? exactInteger(match[0]) : JSON.parse(match[0]);
        if (typeof parsed === "number" && !Number.isFinite(parsed)) reject();
        return parsed;
    }
    const parsed = value(0);
    space();
    if (offset !== text.length) reject();
    return parsed;
}

function exactInteger(token: string): number {
    if (token.length > 128) reject();
    const parts = token.split(/[eE]/);
    const exponent = parts.length === 2 ? Number(parts[1]) : 0;
    if (!Number.isInteger(exponent) || Math.abs(exponent) > 128) reject();
    const negative = parts[0].startsWith('-');
    const mantissa = negative ? parts[0].slice(1) : parts[0];
    const [whole, fraction = ''] = mantissa.split('.');
    let digits = (whole + fraction).replace(/^0+/, '');
    if (!digits) return 0;
    const scale = exponent - fraction.length;
    if (scale >= 0) {
        if (digits.length + scale > 16) reject();
        digits += '0'.repeat(scale);
    } else {
        const keep = digits.length + scale;
        if (keep < 0 || !/^0*$/.test(digits.slice(keep))) reject();
        digits = digits.slice(0, keep);
    }
    const value = Number(digits);
    if (!Number.isSafeInteger(value)) reject();
    return negative ? -value : value;
}
