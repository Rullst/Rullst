(() => {
    'use strict';
    const form = document.getElementById('visibility-report');
    const status = document.getElementById('collection-status');
    if (!form || !status) return;
    let stopped = false;
    let pending = false;
    let last = -Infinity;
    let sequence = Number(form.elements.namedItem('sequence').value);
    let controller;
    const stop = (message) => {
        stopped = true;
        if (controller) controller.abort();
        document.removeEventListener('visibilitychange', report);
        status.textContent = message;
    };
    async function report() {
        if (stopped || pending || performance.now() - last < 1100) return;
        pending = true;
        last = performance.now();
        controller = new AbortController();
        const data = new URLSearchParams(new FormData(form));
        data.set('sequence', String(sequence + 1));
        data.set('event', document.hidden ? 'page_hidden' : 'page_visible');
        try {
            const response = await fetch(form.action, {
                method: 'POST', body: data, credentials: 'same-origin',
                cache: 'no-store', redirect: 'error', signal: controller.signal
            });
            if (stopped) return;
            if (response.status !== 204) {
                stop('Collection stopped. Reload to check your permission and session state.');
                return;
            }
            sequence += 1;
            status.textContent = 'Page visibility report accepted. Reports are not evidence of misconduct.';
        } catch (_) {
            if (!stopped) stop('Collection stopped after a connection error. Reload before continuing.');
        } finally { pending = false; }
    }
    document.querySelectorAll('form[data-session-control]').forEach(control => {
        control.addEventListener('submit', () => stop('Collection stopped on this page. Waiting for the server to confirm your action.'));
    });
    window.addEventListener('pagehide', () => stop('Collection stopped on this page.'), { once: true });
    // Forms expire after five minutes. No automatic renewal or background heartbeat.
    const issued = Number(form.elements.namedItem('issued_at').value) * 1000;
    setTimeout(() => stop('Collection stopped. Reload to renew the session controls.'), Math.max(0, Math.min(issued + 300000, Number(form.dataset.expires) * 1000) - Date.now()));
    document.addEventListener('visibilitychange', report);
})();
