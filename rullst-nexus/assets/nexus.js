// Rullst Nexus browser behaviour. Served same-origin from /nexus/assets so the
// production nonce CSP (script-src 'self') applies: no inline scripts, no
// on*/hx-on attributes and no eval. Handlers are delegated from `document`
// because htmx swaps table, form and chat fragments in place.
(() => {
    "use strict";

    const csrfToken = () => {
        const match = document.cookie.match(/(?:^|;\s*)rullst_csrf=([^;]+)/);
        return match ? match[1] : "";
    };

    document.addEventListener("htmx:configRequest", event => {
        const token = csrfToken();
        if (token) event.detail.headers["X-CSRF-Token"] = token;
    });

    function toast(message, kind) {
        const host = document.getElementById("nexus-toast");
        if (!host) return;
        const safeKind = ["success", "warning", "danger"].includes(kind) ? kind : "success";
        const icon = kind === "success" ? "✅" : kind === "warning" ? "⚠️" : "❌";
        const element = document.createElement("div");
        element.className = "nexus-toast nexus-toast-" + safeKind;
        element.textContent = icon + " " + String(message);
        host.replaceChildren(element);
        setTimeout(() => host.replaceChildren(), 3500);
    }

    const modal = () => document.getElementById("nexus-modal");
    function openModal() {
        const dialog = modal();
        if (dialog && !dialog.open) dialog.showModal();
    }
    function closeModal() {
        const dialog = modal();
        if (dialog && dialog.open) dialog.close();
    }

    function deleteRecord(button) {
        const table = button.dataset.nexusTable || "";
        const id = button.dataset.nexusRecord || "";
        if (!confirm("Are you sure you want to delete record #" + id + "?")) return;
        fetch("/nexus/table/" + encodeURIComponent(table) + "/" + encodeURIComponent(id), {
            method: "DELETE",
            credentials: "same-origin",
            headers: { "X-CSRF-Token": csrfToken() },
        }).then(response => {
            if (response.ok) {
                button.closest("tr")?.remove();
                toast("Record #" + id + " deleted.", "success");
            } else {
                response.text().then(text => toast("Delete failed: " + text, "danger"));
            }
        }).catch(error => toast("Network error: " + error, "danger"));
    }

    // The edit form sends only controls the administrator changed, so an
    // untouched NULL, unregistered or undisplayable value is never rewritten
    // with a widget default. A control counts only after an input/change event:
    // browsers may normalize a default value without any user action.
    function changedNames(form) {
        const names = new Set();
        for (const control of form.elements) {
            if (!control.name || control.dataset.nexusTouched !== "true") continue;
            let changed;
            if (control instanceof HTMLSelectElement) {
                changed = Array.from(control.options).some(option => option.selected !== option.defaultSelected);
            } else if (control.type === "checkbox" || control.type === "radio") {
                changed = control.checked !== control.defaultChecked;
            } else {
                changed = control.value !== control.defaultValue;
            }
            if (changed) names.add(control.name);
        }
        return names;
    }

    function recordBody(form) {
        const data = new FormData(form);
        if (form.dataset.nexusMode !== "edit") return new URLSearchParams(data);
        const names = changedNames(form);
        const body = new URLSearchParams();
        for (const [name, value] of data) {
            if (names.has(name)) body.append(name, value);
        }
        return body;
    }

    const markTouched = event => {
        const control = event.target;
        if (control instanceof HTMLElement && control.closest("form[data-nexus-mode='edit']")) {
            control.dataset.nexusTouched = "true";
        }
    };
    document.addEventListener("input", markTouched);
    document.addEventListener("change", markTouched);

    function saveRecord(button) {
        const form = button.closest("form");
        if (!form) return;
        const params = recordBody(form);
        if (form.dataset.nexusMode === "edit" && !params.toString()) {
            closeModal();
            toast("No changes to save.", "warning");
            return;
        }
        const body = params.toString();
        const restore = () => {
            button.disabled = false;
            button.textContent = "Save Record";
        };
        button.disabled = true;
        button.textContent = "Saving...";
        fetch(form.dataset.nexusAction || "", {
            method: "POST",
            credentials: "same-origin",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
                "X-CSRF-Token": csrfToken(),
            },
            body,
        }).then(response => {
            restore();
            if (response.ok) {
                closeModal();
                toast("Saved successfully!", "success");
                window.htmx?.ajax("GET", window.location.pathname, { target: "#nexus-content", swap: "innerHTML" });
            } else {
                response.text().then(text => toast("Save failed: " + text, "danger"));
            }
        }).catch(error => {
            restore();
            toast("Network error: " + error, "danger");
        });
    }

    document.addEventListener("click", event => {
        const target = event.target instanceof Element ? event.target : null;
        if (!target) return;
        const remove = target.closest("[data-nexus-delete]");
        if (remove) return deleteRecord(remove);
        const save = target.closest("[data-nexus-save]");
        if (save) return saveRecord(save);
        if (target.closest("[data-nexus-modal-close]")) return closeModal();
        const prompt = target.closest("[data-nexus-prompt]");
        const input = document.getElementById("nexus-chat-input");
        if (prompt && input) {
            input.value = prompt.dataset.nexusPrompt || "";
            input.focus();
        }
    });

    document.addEventListener("change", event => {
        const box = event.target;
        if (!(box instanceof HTMLInputElement) || !box.matches("[data-nexus-select-all]")) return;
        (box.form || document).querySelectorAll(".nexus-batch-check").forEach(check => {
            check.checked = box.checked;
        });
    });

    document.addEventListener("submit", event => {
        const form = event.target;
        if (!(form instanceof HTMLFormElement)) return;
        if (form.matches("[data-nexus-no-submit]")) {
            event.preventDefault();
            return;
        }
        const question = form.dataset.nexusConfirm;
        if (question && !confirm(question)) event.preventDefault();
    });

    document.addEventListener("htmx:afterSwap", event => {
        if (event.detail?.target?.id === "nexus-modal-body") openModal();
    });

    document.addEventListener("htmx:afterRequest", event => {
        const source = event.detail?.elt;
        if (!(source instanceof HTMLFormElement) || !source.matches(".nexus-chat-form")) return;
        if (!event.detail.successful) return;
        source.reset();
        const messages = document.getElementById("nexus-chat-messages");
        if (messages) messages.scrollTop = messages.scrollHeight;
    });
})();

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
