// Dropdown menus, copied by `cargo rullst ui add menu`; the application owns
// this file. Loaded as a same-origin module, which Rullst's
// Content-Security-Policy allows without a nonce; it never evaluates strings
// and sets styles through the CSSOM only.
//
// The menu is a `popover` opened by its button's `popovertarget`; the browser
// closes it on Escape and on a click elsewhere. This module places it under
// the button, keeps `aria-expanded` in step and adds the keyboard of the
// ARIA menu button pattern. Events are delegated from `document`, so menus
// that htmx swaps in work without setup.
const MENU = '[role="menu"][popover]';
const ITEM = '[role="menuitem"]:not([disabled]):not([aria-disabled="true"])';
const GAP = 4;
const MARGIN = 8;

function items(menu) {
  return [...menu.querySelectorAll(ITEM)];
}

function buttonOf(menu) {
  return menu.id ? document.querySelector(`[popovertarget="${CSS.escape(menu.id)}"]`) : null;
}

function place(menu, button) {
  const anchor = button.getBoundingClientRect();
  menu.style.position = "fixed";
  menu.style.inset = "auto";
  menu.style.margin = "0";
  const width = menu.offsetWidth;
  const height = menu.offsetHeight;
  let top = anchor.bottom + GAP;
  if (top + height > window.innerHeight - MARGIN && anchor.top - GAP - height >= MARGIN) {
    top = anchor.top - GAP - height;
  }
  const rtl = getComputedStyle(button).direction === "rtl";
  let left = rtl ? anchor.right - width : anchor.left;
  left = Math.max(MARGIN, Math.min(left, window.innerWidth - width - MARGIN));
  menu.style.top = `${Math.max(MARGIN, top)}px`;
  menu.style.left = `${left}px`;
}

// An open menu sits under its button, which says so.
function opened(menu, button) {
  button.setAttribute("aria-expanded", "true");
  place(menu, button);
}

// `toggle` does not bubble: listen while it is captured on its way down. It
// arrives after the menu opened, so it never takes focus back from an item
// the keyboard already reached (ArrowDown on the button focuses one at once).
document.addEventListener(
  "toggle",
  (event) => {
    const menu = event.target;
    if (!(menu instanceof HTMLElement) || !menu.matches(MENU)) return;
    const button = buttonOf(menu);
    if (event.newState === "open") {
      if (button) opened(menu, button);
      if (!menu.contains(document.activeElement)) items(menu)[0]?.focus();
    } else {
      button?.setAttribute("aria-expanded", "false");
      const active = document.activeElement;
      if (button && (!active || active === document.body || menu.contains(active))) button.focus();
    }
  },
  true,
);

function moveFocus(menu, current, key) {
  const list = items(menu);
  if (list.length === 0) return false;
  const index = list.indexOf(current);
  const next = {
    ArrowDown: list[(index + 1) % list.length],
    ArrowUp: list[(index - 1 + list.length) % list.length],
    Home: list[0],
    End: list.at(-1),
  }[key];
  if (!next) return false;
  next.focus();
  return true;
}

document.addEventListener("keydown", (event) => {
  if (!(event.target instanceof Element) || event.altKey || event.ctrlKey || event.metaKey) return;
  const menu = event.target.closest(MENU);
  if (menu) {
    if (event.key === "Tab") {
      // Focus moves on with the default Tab; the menu closes behind it.
      menu.hidePopover();
    } else if (moveFocus(menu, event.target, event.key)) {
      event.preventDefault();
    }
    return;
  }
  const button = event.target.closest("[popovertarget]");
  if (!button || (event.key !== "ArrowDown" && event.key !== "ArrowUp")) return;
  const controlled = document.getElementById(button.getAttribute("popovertarget") || "");
  if (!controlled?.matches(MENU)) return;
  event.preventDefault();
  if (!controlled.matches(":popover-open")) {
    // A click keeps the button as the popover's invoker; the menu opens at
    // once, so its item takes focus now, before the next key.
    button.click();
    if (!controlled.matches(":popover-open")) return;
    opened(controlled, button);
  }
  moveFocus(controlled, null, event.key === "ArrowUp" ? "End" : "Home");
});

// Choosing an item closes the menu; links and submit buttons then act.
document.addEventListener("click", (event) => {
  const item = event.target instanceof Element ? event.target.closest(ITEM) : null;
  const menu = item?.closest(MENU);
  if (menu?.matches(":popover-open")) menu.hidePopover();
});

function placeOpenMenus() {
  for (const menu of document.querySelectorAll(`${MENU}:popover-open`)) {
    const button = buttonOf(menu);
    if (button) place(menu, button);
  }
}

window.addEventListener("resize", placeOpenMenus, { passive: true });
window.addEventListener("scroll", placeOpenMenus, { capture: true, passive: true });
