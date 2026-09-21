import { ContractClient, ContractError, DtoLessonRequest } from './client';
// No Node type package: this fixture only needs argv, exit status and timers.
declare const process: { argv: string[]; exitCode: number };
function assert(condition: unknown, message: string): asserts condition { if (!condition) throw new Error(message); }
async function fails(run: () => Promise<unknown>): Promise<void> {
    let failed = false;
    try { await run(); } catch (error) { assert(error instanceof ContractError, 'must redact transport/payload error'); failed = true; }
    assert(failed, 'invalid contract accepted');
}
async function main(): Promise<void> {
    const base = process.argv[2];
    const client = new ContractClient(base, () => 'fixture_alice');
    const body: DtoLessonRequest = { title:'Olá 🌍', note:null, attempts:1, tags:['a'], active:true, details:{lower:-9007199254740991,upper:9007199254740991,maybe_number:null,maybe_flag:null,maybe_items:[null,true,false],child:{code:'nested'}} };
    const response = await client.call_save_lesson({p_owner:'alice',q_verbose:true,q_limit:12,body});
    assert(response.status === 200, 'authenticated save');
    assert(response.body.owner === 'alice' && response.body.request.title === body.title, 'Unicode/ownership round trip');
    assert(response.body.request.note === null && !('nickname' in response.body.request), 'null differs from absent');
    assert(response.body.request.details?.upper === 9007199254740991 && response.body.request.details.maybe_items?.[0] === null, 'safe integer limits and nullable nested items');
    const read = await client.call_read_lesson({p_owner:'alice'});
    assert(read.status === 200 && read.body.request.active, 'typed GET');
    const denied = await client.call_save_lesson({p_owner:'bob',body});
    assert(denied.status === 403 && denied.body.code === 'denied', 'cross-owner access denied');
    const anonymous = await new ContractClient(base).call_read_lesson({p_owner:'alice'});
    assert(anonymous.status === 401, 'unauthenticated access denied');
    const raw = await fetch(base + '/lessons/alice?limit=1&limit=2', {method:'POST',headers:{Authorization:'Bearer fixture_alice','Content-Type':'application/json'},body:JSON.stringify(body)});
    assert(raw.status === 422, 'server rejects repeated query parameters');
    // Compile-time negative controls must remain errors under strict nullability.
    if (false) {
        // @ts-expect-error A required nullable field cannot be omitted.
        const missing: DtoLessonRequest = { title:'x', attempts:1, tags:[], active:true };
        // @ts-expect-error Optional non-nullable fields cannot be null.
        const nullOptional: DtoLessonRequest = { ...body, nickname:null };
        // @ts-expect-error Optional values use omission, not explicit undefined.
        const undefinedOptional: DtoLessonRequest = { ...body, nickname:undefined };
        // @ts-expect-error The parameter is an integer.
        await client.call_read_lesson({p_owner:'alice',q_limit:'1'});
        void missing; void nullOptional; void undefinedOptional;
    }
    const forced = (value: unknown) => value as DtoLessonRequest;
    await fails(() => client.call_save_lesson({p_owner:'alice',body:forced({...body,note:undefined})}));
    await fails(() => client.call_save_lesson({p_owner:'alice',body:forced({...body,nickname:null})}));
    await fails(() => client.call_save_lesson({p_owner:'alice',body:{...body,attempts:101}}));
    await fails(() => client.call_save_lesson({p_owner:'alice',body:{...body,title:String.fromCharCode(0xd800)}}));
    await fails(() => client.call_read_lesson({p_owner:'..'}));
    const getter = Object.defineProperty({...body}, 'title', {get() { throw new Error('private diagnostic'); }, enumerable:true});
    await fails(() => client.call_save_lesson({p_owner:'alice',body:getter as DtoLessonRequest}));
    const fetchOriginal = globalThis.fetch;
    let requested = '';
    const encoded = JSON.stringify({owner:'alice',request:body});
    try {
        globalThis.fetch = async (url, options) => {
            requested = String(url);
            assert(options?.redirect === 'error' && options.credentials === 'omit', 'transport boundaries');
            return new Response(encoded,{status:200,headers:{'Content-Type':'application/json'}});
        };
        await client.call_read_lesson({p_owner:'a/b?c#d%',q_limit:12});
        assert(requested.endsWith('/lessons/a%2Fb%3Fc%23d%25?limit=12'), 'single encoded path segment');
        const invalidResponses = [
            new Response('private upstream body',{status:502}),
            new Response(encoded,{status:200,headers:{'Content-Type':'text/html'}}),
            new Response(encoded.replace('"owner":"alice"','"owner":"alice","owner":"bob"'),{status:200,headers:{'Content-Type':'application/json'}}),
            new Response(encoded.replace('"attempts":1','"attempts":9007199254740992'),{status:200,headers:{'Content-Type':'application/json'}}),
            new Response(encoded.replace('"attempts":1','"attempts":1.00000000000000001'),{status:200,headers:{'Content-Type':'application/json'}}),
            new Response(' '.repeat(65537),{status:200,headers:{'Content-Type':'application/json'}}),
            new Response(new Uint8Array([0xff]),{status:200,headers:{'Content-Type':'application/json'}}),
            new Response(encoded,{status:302,headers:{'Content-Type':'application/json','Location':'https://example.invalid/private'}}),
        ];
        for (const invalid of invalidResponses) {
            globalThis.fetch = async () => invalid;
            await fails(() => client.call_read_lesson({p_owner:'alice'}));
        }
    } finally { globalThis.fetch = fetchOriginal; }
    const abort = new AbortController(); abort.abort();
    await fails(() => client.call_read_lesson({p_owner:'alice'},abort.signal));
    const delayed = new ContractClient(base, () => new Promise<string>(() => {}));
    const cancel = new AbortController();
    const pending = fails(() => delayed.call_read_lesson({p_owner:'alice'},cancel.signal));
    setTimeout(() => cancel.abort(),20);
    await pending;
    console.log('TypeScript HTTP contract and adversarial transport assertions passed');
}
main().catch(error => { console.error(error); process.exitCode = 1; });
