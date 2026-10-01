// Small same-origin behaviors for the Rullst blog showcase.
//
// The production Content Security Policy blocks inline `onclick` handlers, so
// controls declare `data-action` (and `data-target` when they act on another
// element) and this module handles them through one delegated listener.

const actions = {
  'toggle-drawer'(control) {
    const drawer = document.getElementById(control.dataset.target);
    if (!drawer) return;
    const open = drawer.classList.toggle('open');
    control.setAttribute('aria-expanded', String(open));
  },
  'open-dialog'(control) {
    const dialog = document.getElementById(control.dataset.target);
    if (dialog instanceof HTMLDialogElement) dialog.showModal();
  },
  'close-dialog'(control) {
    const dialog = document.getElementById(control.dataset.target);
    if (dialog instanceof HTMLDialogElement) dialog.close();
  },
  'advance-progress'(control) {
    const progress = document.getElementById(control.dataset.target);
    if (progress instanceof HTMLProgressElement) {
      progress.value = progress.value >= 100 ? 20 : progress.value + 15;
    }
  },
};

document.addEventListener('click', (event) => {
  const control = event.target instanceof Element ? event.target.closest('[data-action]') : null;
  const action = control ? actions[control.dataset.action] : undefined;
  if (action) action(control);
});
