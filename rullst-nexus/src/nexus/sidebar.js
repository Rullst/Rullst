// Progressive enhancement: without this script mobile navigation stays visible.
document.addEventListener("DOMContentLoaded", () => {
    const sidebar = document.getElementById("nexus-sidebar");
    const toggle = document.querySelector(".nexus-topbar-toggle");
    const close = document.getElementById("nexus-sidebar-close");
    const backdrop = document.getElementById("nexus-sidebar-backdrop");
    const main = document.querySelector(".nexus-main");
    if (!sidebar || !toggle || !close || !backdrop || !main) return;
    const mobile = matchMedia("(max-width: 900px)");
    let open = false;
    const focusable = () => Array.from(sidebar.querySelectorAll(
        "a[href], button:not([disabled]), [tabindex='0']"
    )).filter(element => element.getClientRects().length > 0);

    function setOpen(next, restoreFocus = true) {
        open = mobile.matches && next;
        sidebar.classList.toggle("nexus-sidebar-open", open);
        toggle.setAttribute("aria-expanded", String(open));
        backdrop.hidden = !open;
        main.inert = open;
        sidebar.inert = mobile.matches && !open;
        if (mobile.matches) sidebar.setAttribute("aria-hidden", String(!open));
        else sidebar.removeAttribute("aria-hidden");
        if (open) {
            sidebar.setAttribute("role", "dialog");
            sidebar.setAttribute("aria-modal", "true");
            close.focus();
        } else {
            sidebar.removeAttribute("role");
            sidebar.removeAttribute("aria-modal");
            if (restoreFocus && mobile.matches) toggle.focus();
        }
    }
    toggle.addEventListener("click", () => setOpen(!open));
    close.addEventListener("click", () => setOpen(false));
    backdrop.addEventListener("click", () => setOpen(false));
    sidebar.addEventListener("click", event => {
        if (open && event.target instanceof Element && event.target.closest("a[href]")) {
            setOpen(false);
        }
    });
    document.addEventListener("keydown", event => {
        if (!open) return;
        if (event.key === "Escape") {
            event.preventDefault();
            setOpen(false);
        } else if (event.key === "Tab") {
            const targets = focusable();
            const first = targets[0];
            const last = targets[targets.length - 1];
            if (event.shiftKey && (document.activeElement === first || !sidebar.contains(document.activeElement))) {
                event.preventDefault();
                last?.focus();
            } else if (!event.shiftKey && (document.activeElement === last || !sidebar.contains(document.activeElement))) {
                event.preventDefault();
                first?.focus();
            }
        }
    });
    document.addEventListener("focusin", event => {
        if (open && !sidebar.contains(event.target)) close.focus();
    });
    function resize() {
        const inside = sidebar.contains(document.activeElement);
        const onClose = document.activeElement === close;
        setOpen(false, false);
        if (mobile.matches && inside) toggle.focus();
        else if (!mobile.matches && onClose) sidebar.querySelector("a[href]")?.focus();
    }
    mobile.addEventListener("change", resize);
    document.documentElement.classList.add("nexus-drawer-ready");
    resize();
});
