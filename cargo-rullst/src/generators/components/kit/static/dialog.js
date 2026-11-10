// Dialog commands, copied by `cargo rullst ui add dialog`; the application
// owns this file. Loaded as a same-origin module, which Rullst's
// Content-Security-Policy allows without a nonce; it never evaluates strings.
//
// Buttons open and close dialogs with the HTML invoker commands
// (`commandfor="id"` and `command="show-modal"`, `"close"` or
// `"request-close"`), built into Chrome 135, Firefox 144 and Safari 26.2. In
// older browsers this module runs the same commands (and the popover ones,
// `show-popover`, `hide-popover`, `toggle-popover`). In every browser it
// returns focus to the button that opened a dialog when the dialog closes,
// if focus would otherwise be lost.
const native = "commandForElement" in HTMLButtonElement.prototype;
const openers = new WeakMap();

function target(button) {
  const id = button.getAttribute("commandfor");
  return id ? document.getElementById(id) : null;
}

function runDialog(dialog, command, button) {
  if (command === "show-modal" && !dialog.open) {
    dialog.showModal();
  } else if (command === "close" && dialog.open) {
    dialog.close(button.value);
  } else if (command === "request-close" && dialog.open) {
    if (typeof dialog.requestClose === "function") {
      dialog.requestClose(button.value);
    } else if (dialog.dispatchEvent(new Event("cancel", { cancelable: true }))) {
      dialog.close(button.value);
    }
  }
}

function runPopover(element, command) {
  const open = element.matches(":popover-open");
  if (command === "show-popover" && !open) element.showPopover();
  else if (command === "hide-popover" && open) element.hidePopover();
  else if (command === "toggle-popover") element.togglePopover();
}

document.addEventListener("click", (event) => {
  const button = event.target instanceof Element ? event.target.closest("button[commandfor]") : null;
  if (!button || button.disabled) return;
  const element = target(button);
  if (!element) return;
  const command = (button.getAttribute("command") || "").toLowerCase();
  if (element instanceof HTMLDialogElement && command === "show-modal") openers.set(element, button);
  if (native) return;
  if (element instanceof HTMLDialogElement) runDialog(element, command, button);
  else if (element.hasAttribute("popover")) runPopover(element, command);
});

// `close` does not bubble: listen while it is captured on its way down.
document.addEventListener(
  "close",
  (event) => {
    const dialog = event.target;
    if (!(dialog instanceof HTMLDialogElement)) return;
    const opener = openers.get(dialog);
    openers.delete(dialog);
    const active = document.activeElement;
    const lost = !active || active === document.body || dialog.contains(active);
    if (opener && opener.isConnected && lost) opener.focus();
  },
  true,
);
