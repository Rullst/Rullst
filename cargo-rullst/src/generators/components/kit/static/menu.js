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
// Which item to focus when a menu opens: "first" or "last".
const focusOnOpen = new WeakMap();

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

// `toggle` does not bubble: listen while it is captured on its way down.
document.addEventListener(
  "toggle",
  (event) => {
    const menu = event.target;
    if (!(menu instanceof HTMLElement) || !menu.matches(MENU)) return;
    const button = buttonOf(menu);
    if (event.newState === "open") {
      button?.setAttribute("aria-expanded", "true");
      if (button) place(menu, button);
      const list = items(menu);
      const first = focusOnOpen.get(menu) === "last" ? list[list.length - 1] : list[0];
      focusOnOpen.delete(menu);
      first?.focus();
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
    End: list[list.length - 1],
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
  if (!controlled || !controlled.matches(MENU)) return;
  event.preventDefault();
  if (controlled.matches(":popover-open")) {
    moveFocus(controlled, null, event.key === "ArrowUp" ? "End" : "Home");
  } else {
    focusOnOpen.set(controlled, event.key === "ArrowUp" ? "last" : "first");
    // A click keeps the button as the popover's invoker.
    button.click();
  }
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
