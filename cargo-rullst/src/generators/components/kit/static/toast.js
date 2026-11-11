// Toast notifications, copied by `cargo rullst ui add toast`; the application
// owns this file. Loaded as a same-origin module, which Rullst's
// Content-Security-Policy allows without a nonce; it never evaluates strings
// and writes messages as text, never as HTML.
//
// A `ui:toast` event shows a toast. htmx fires it from a response header
// `HX-Trigger: {"ui:toast":{"message":"Saved","tone":"success"}}` (see
// `src/ui/toast.rs`); `{"ui:toast":"Saved"}` shows an info toast. The toast
// is a copy of the region's `<template>` for its tone, added to the
// `aria-live="polite"` list. Info and success toasts leave after
// `DISMISS_AFTER` milliseconds unless hovered or focused; warnings and errors
// stay until their close button is pressed.
const DISMISS_AFTER = 6000;
const TONES = new Set(["info", "success", "warning", "error"]);

function schedule(toast) {
  if (toast.dataset.tone === "warning" || toast.dataset.tone === "error") return;
  let timer = setTimeout(() => toast.remove(), DISMISS_AFTER);
  const pause = () => clearTimeout(timer);
  const resume = () => {
    if (toast.matches(":hover") || toast.contains(document.activeElement)) return;
    clearTimeout(timer);
    timer = setTimeout(() => toast.remove(), DISMISS_AFTER);
  };
  toast.addEventListener("mouseenter", pause);
  toast.addEventListener("focusin", pause);
  toast.addEventListener("mouseleave", resume);
  toast.addEventListener("focusout", () => setTimeout(resume, 0));
}

function show(detail) {
  const list = document.querySelector("[data-ui-toasts]");
  if (!list) return;
  const message = typeof detail === "string" ? detail : String(detail?.message ?? detail?.value ?? "");
  if (!message) return;
  const tone = TONES.has(detail?.tone) ? detail.tone : "info";
  const template = list.parentElement?.querySelector(`template[data-ui-toast-template="${tone}"]`);
  const toast = template?.content.firstElementChild?.cloneNode(true);
  const text = toast?.querySelector("[data-ui-toast-message]");
  if (!toast || !text) return;
  text.textContent = message;
  list.append(toast);
  schedule(toast);
}

document.addEventListener("ui:toast", (event) => show(event.detail));

document.addEventListener("click", (event) => {
  const close = event.target instanceof Element ? event.target.closest("[data-ui-toast-close]") : null;
  close?.closest("[data-ui-toast]")?.remove();
});

// Toasts the server rendered with the page.
for (const toast of document.querySelectorAll("[data-ui-toasts] [data-ui-toast]")) schedule(toast);
