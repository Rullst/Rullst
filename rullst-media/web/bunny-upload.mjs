// TUS 1.0 creation/HEAD/PATCH for Bunny Stream. No credentials or file contents
// enter persistent browser storage. Retain this object to resume the SAME File.
// The application owns authentication/CSRF for getGrant and remote cancellation.
export class BunnyUpload {
  #file; #getGrant; #progress; #fetch; #fixture; #title; #mime;
  #grant; #url; #binding; #offset = 0; #status = 'idle'; #running = false;
  #controller; #creationAttempted = false; #requests = 0;

  constructor({ file, title, getGrant, onProgress = () => {}, maxBytes = 1073741824,
    allowProtocolFixture = false, fetchImpl = globalThis.fetch.bind(globalThis) }) {
    if (!(file instanceof Blob) || file.size < 1 || !Number.isSafeInteger(maxBytes)
      || maxBytes < 1 || maxBytes > 1073741824 || file.size > maxBytes
      || typeof title !== 'string' || !title.trim() || new TextEncoder().encode(title).length > 200
      || /[\x00-\x1f<>]/.test(title) || typeof getGrant !== 'function'
      || typeof onProgress !== 'function' || typeof fetchImpl !== 'function'
      || !['video/mp4', 'video/webm', 'video/quicktime'].includes(file.type)) {
      throw new Error('invalid_upload_input');
    }
    this.#file = file; this.#title = title; this.#mime = file.type;
    this.#getGrant = getGrant; this.#progress = onProgress; this.#fixture = allowProtocolFixture === true;
    this.#fetch = fetchImpl;
  }

  get state() { return Object.freeze({ status: this.#status, uploaded: this.#offset, total: this.#file?.size ?? 0 }); }
  pause() {
    if (this.#status === 'complete' || this.#status === 'cancelled') return;
    this.#status = 'paused'; this.#controller?.abort(); this.#grant = undefined;
  }
  // Stops local transfer. The app must separately call its authorized deletion
  // endpoint if it intends to remove the provider asset or cancel access.
  cancel() { this.#status = 'cancelled'; this.#controller?.abort(); this.#grant = undefined; this.#file = undefined; }

  async start() {
    if (this.#running) throw new Error('upload_busy');
    if (this.#status === 'complete') return this.state;
    if (this.#status === 'cancelled') throw new Error('upload_cancelled');
    if (this.#creationAttempted && !this.#url) throw new Error('upload_creation_uncertain');
    this.#running = true; this.#status = 'uploading'; this.#controller = new AbortController();
    const totalTimer = setTimeout(() => this.#controller?.abort(), 900000);
    try {
      await this.#authorize();
      if (!this.#url) await this.#create();
      await this.#head();
      while (this.#offset < this.#file.size) {
        this.#checkActive();
        let delivered = false;
        for (let attempt = 0; attempt < 3 && !delivered; attempt++) {
          await this.#authorize();
          const start = this.#offset;
          const end = Math.min(start + 1048576, this.#file.size);
          try {
            const response = await this.#request(this.#url, 'PATCH', {
              'Content-Type': 'application/offset+octet-stream', 'Upload-Offset': String(start),
            }, this.#file.slice(start, end));
            if (response.status === 401) this.#grant = undefined;
            if (response.status !== 204) throw new Error('upload_chunk_rejected');
            this.#version(response);
            if (this.#number(response, 'Upload-Offset') !== end) throw new Error('upload_offset_invalid');
            this.#offset = end; delivered = true;
          } catch (error) {
            this.#checkActive();
            if (error.message === 'upload_offset_invalid' || error.message === 'upload_version_invalid') throw error;
            // A lost PATCH response may have committed some/all bytes. HEAD,
            // never blind replay, establishes the next safe offset.
            await this.#authorize(); await this.#head();
            delivered = this.#offset >= end;
            if (!delivered && attempt === 2) throw new Error('upload_retry_exhausted');
          }
        }
        this.#progress(this.state);
      }
      this.#status = 'complete'; this.#grant = undefined; this.#progress(this.state);
      return this.state;
    } catch {
      if (this.#status === 'paused' || this.#status === 'cancelled') return this.state;
      this.#status = this.#creationAttempted && !this.#url ? 'uncertain' : 'paused';
      this.#grant = undefined;
      throw new Error(this.#status === 'uncertain' ? 'upload_creation_uncertain' : 'upload_interrupted');
    } finally {
      clearTimeout(totalTimer); this.#running = false; this.#controller = undefined;
    }
  }

  #checkActive() {
    if (this.#status !== 'uploading' || this.#controller?.signal.aborted || !this.#file) throw new Error('upload_stopped');
  }

  async #authorize() {
    this.#checkActive();
    if (this.#grant && this.#grant.expires_at > Math.floor(Date.now() / 1000) + 10) return;
    const signal = AbortSignal.any([this.#controller.signal, AbortSignal.timeout(30000)]);
    const grant = await this.#race(this.#getGrant({ signal }), signal);
    this.#checkActive();
    let endpoint;
    try { endpoint = new URL(grant.endpoint); } catch { throw new Error('upload_grant_invalid'); }
    const now = Math.floor(Date.now() / 1000);
    const remote = grant.mode === 'RemoteUnvalidated' && endpoint.href === 'https://video.bunnycdn.com/tusupload';
    const local = this.#fixture && grant.mode === 'ProtocolFixture' && endpoint.protocol === 'http:'
      && ['127.0.0.1', '[::1]'].includes(endpoint.hostname) && endpoint.port !== '0' && endpoint.pathname === '/tusupload';
    if ((!remote && !local) || endpoint.username || endpoint.password || endpoint.search || endpoint.hash
      || !Number.isSafeInteger(grant.library) || grant.library < 1
      || !/^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(grant.video)
      || !/^[0-9a-f]{64}$/.test(grant.signature) || !Number.isSafeInteger(grant.expires_at)
      || grant.expires_at <= now || grant.expires_at > now + 3600) throw new Error('upload_grant_invalid');
    // The existing upload URL includes no authority to change video/library.
    const binding = `${endpoint.href}|${grant.library}|${grant.video}`;
    if (this.#binding && this.#binding !== binding) throw new Error('upload_binding_changed');
    this.#binding = binding;
    this.#grant = { ...grant, endpoint: endpoint.href };
  }

  async #create() {
    const base64 = value => btoa(String.fromCharCode(...new TextEncoder().encode(value)));
    this.#creationAttempted = true;
    const response = await this.#request(this.#grant.endpoint, 'POST', {
      'Upload-Length': String(this.#file.size),
      'Upload-Metadata': `filetype ${base64(this.#mime)},title ${base64(this.#title)}`,
    });
    if (response.status !== 201) throw new Error('upload_creation_uncertain');
    this.#version(response);
    const location = response.headers.get('Location');
    if (!location || location.length > 2048) throw new Error('upload_location_invalid');
    const target = new URL(location, this.#grant.endpoint);
    const origin = new URL(this.#grant.endpoint).origin;
    if (target.origin !== origin || !target.pathname.startsWith('/tusupload/')
      || target.username || target.password || target.search || target.hash
      || /%2f|%5c|%2e/i.test(target.pathname)) throw new Error('upload_location_invalid');
    this.#url = target.href;
  }

  async #head() {
    const response = await this.#request(this.#url, 'HEAD');
    if (![200, 204].includes(response.status)) throw new Error('upload_resume_rejected');
    this.#version(response);
    const offset = this.#number(response, 'Upload-Offset');
    if (this.#number(response, 'Upload-Length') !== this.#file.size || offset < this.#offset || offset > this.#file.size) throw new Error('upload_offset_invalid');
    this.#offset = offset;
  }

  #version(response) { if (response.headers.get('Tus-Resumable') !== '1.0.0') throw new Error('upload_version_invalid'); }
  #number(response, header) {
    const value = response.headers.get(header);
    if (!value || !/^(0|[1-9][0-9]{0,15})$/.test(value) || !Number.isSafeInteger(Number(value))) throw new Error('upload_offset_invalid');
    return Number(value);
  }

  async #request(url, method, headers = {}, body) {
    this.#checkActive();
    if (++this.#requests > 4096) throw new Error('upload_request_budget');
    const grant = this.#grant;
    const signal = AbortSignal.any([this.#controller.signal, AbortSignal.timeout(30000)]);
    const response = await this.#race(this.#fetch(url, {
      method, body, headers: { ...headers, 'Tus-Resumable': '1.0.0', AuthorizationSignature: grant.signature,
        AuthorizationExpire: String(grant.expires_at), LibraryId: String(grant.library), VideoId: grant.video },
      credentials: 'omit', redirect: 'error', referrerPolicy: 'no-referrer', cache: 'no-store', signal,
    }), signal);
    this.#checkActive();
    if (response.body) await this.#race(response.body.cancel(), signal);
    return response;
  }

  #race(promise, signal) {
    if (signal.aborted) return Promise.reject(new Error('upload_stopped'));
    return new Promise((resolve, reject) => {
      const abort = () => reject(new Error('upload_stopped'));
      signal.addEventListener('abort', abort, { once: true });
      Promise.resolve(promise).then(resolve, reject).finally(() => signal.removeEventListener('abort', abort));
    });
  }
}
