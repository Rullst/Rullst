(() => {
    'use strict';
    const form = document.getElementById('visibility-report');
    const status = document.getElementById('collection-status');
    if (!form || !status) return;
    const selected = Number(form.dataset.collection);
    const deadline = Math.min(Number(form.elements.namedItem('issued_at').value) * 1000 + 300000,
        Number(form.dataset.expires) * 1000);
    let sequence = Number(form.elements.namedItem('sequence').value);
    let stopped = false, pending = false, last = -Infinity, timer, expiry, controller;
    const queue = [], listeners = [];
    const listen = (target, name, listener) => {
        target.addEventListener(name, listener);
        listeners.push([target, name, listener]);
    };
    const stop = message => {
        if (stopped) return;
        stopped = true;
        clearTimeout(timer); clearTimeout(expiry); queue.length = 0;
        controller?.abort();
        for (const [target, name, listener] of listeners) target.removeEventListener(name, listener);
        status.textContent = message;
    };
    const expired = () => {
        if (Date.now() < deadline) return false;
        stop('Collection stopped. Reload to renew the session controls.'); return true;
    };
    if (!Number.isInteger(selected) || selected < 0 || selected > 15 ||
        !Number.isSafeInteger(sequence) || sequence < 0 || !Number.isFinite(deadline)) {
        stop('Collection stopped because the page configuration is invalid.'); return;
    }
    async function send() {
        if (stopped || pending || expired() || queue.length === 0) return;
        const remaining = 1100 - (performance.now() - last);
        if (remaining > 0) { clearTimeout(timer); timer = setTimeout(send, remaining); return; }
        if (!Number.isSafeInteger(sequence + 1)) { stop('Collection stopped at its sequence limit.'); return; }
        pending = true; last = performance.now(); controller = new AbortController();
        const data = new URLSearchParams(new FormData(form));
        data.set('sequence', String(sequence + 1)); data.set('event', queue.shift());
        try {
            const response = await fetch(form.action, {
                method: 'POST', body: data, credentials: 'same-origin',
                cache: 'no-store', redirect: 'error', signal: controller.signal
            });
            if (stopped) return;
            if (response.status !== 204) {
                stop('Collection stopped. Reload to check your permission and session state.'); return;
            }
            sequence += 1;
            status.textContent = 'Observation accepted. Reports are not evidence of misconduct.';
        } catch (_) {
            if (!stopped) stop('Collection stopped after a connection error. Reload before continuing.');
        } finally { pending = false; if (!stopped) void send(); }
    }
    const report = (event, name) => {
        if (stopped || !event.isTrusted || expired()) return;
        if (queue.length >= 16) { stop('Collection stopped after too many events. Reload to check the session.'); return; }
        queue.push(name); void send();
    };
    if (selected & 1) listen(document, 'visibilitychange', event => report(event, document.hidden ? 'page_hidden' : 'page_visible'));
    if (selected & 2) {
        listen(window, 'focus', event => report(event, 'window_focused'));
        listen(window, 'blur', event => report(event, 'window_blurred'));
    }
    if (selected & 4) for (const name of ['copy', 'cut', 'paste']) {
        // Never inspect clipboardData, selection, key contents or copied text.
        listen(document, name, event => report(event, name + '_attempt'));
    }
    if (selected & 8) listen(document, 'fullscreenchange', event => report(event, document.fullscreenElement ? 'fullscreen_entered' : 'fullscreen_exited'));
    document.querySelectorAll('form[data-session-control]').forEach(control => {
        listen(control, 'submit', () => stop('Collection stopped on this page. Waiting for the server to confirm your action.'));
    });
    listen(window, 'pagehide', () => stop('Collection stopped on this page.'));
    // No automatic permission renewal, capture request or background heartbeat.
    if (!expired()) expiry = setTimeout(expired, Math.max(0, deadline - Date.now()));
})();
