// Tabs, copied by `cargo rullst ui add tabs`; the application owns this
// file. Loaded as a same-origin module, which Rullst's
// Content-Security-Policy allows without a nonce; it never evaluates strings.
//
// The server renders the selected tab; this module switches tabs following
// the ARIA tabs pattern with automatic activation: a click, or
// ArrowLeft/ArrowRight (ArrowUp/ArrowDown with aria-orientation="vertical"),
// Home and End on the focused tab. Only the selected tab is in the Tab
// order. Events are delegated from `document`, so tabs that htmx swaps in
// work without setup.
const TAB = '[role="tab"]';

function tabsOf(list) {
  return [...list.querySelectorAll(TAB)].filter((tab) => tab.closest('[role="tablist"]') === list);
}

function select(tab, focus) {
  const list = tab.closest('[role="tablist"]');
  if (!list) return;
  for (const other of tabsOf(list)) {
    const selected = other === tab;
    other.setAttribute("aria-selected", String(selected));
    other.tabIndex = selected ? 0 : -1;
    const panel = document.getElementById(other.getAttribute("aria-controls") || "");
    if (panel) panel.hidden = !selected;
  }
  if (focus) tab.focus();
}

// The keys that move to the next and the previous tab.
function directions(list) {
  if (list.getAttribute("aria-orientation") === "vertical") return ["ArrowDown", "ArrowUp"];
  if (getComputedStyle(list).direction === "rtl") return ["ArrowLeft", "ArrowRight"];
  return ["ArrowRight", "ArrowLeft"];
}

document.addEventListener("click", (event) => {
  const tab = event.target instanceof Element ? event.target.closest(TAB) : null;
  if (tab?.closest('[role="tablist"]')) select(tab, false);
});

document.addEventListener("keydown", (event) => {
  if (event.altKey || event.ctrlKey || event.metaKey) return;
  const tab = event.target instanceof Element ? event.target.closest(TAB) : null;
  const list = tab?.closest('[role="tablist"]');
  if (!list) return;
  const tabs = tabsOf(list);
  const index = tabs.indexOf(tab);
  const [forward, backward] = directions(list);
  let next = null;
  if (event.key === forward) next = tabs[(index + 1) % tabs.length];
  else if (event.key === backward) next = tabs[(index - 1 + tabs.length) % tabs.length];
  else if (event.key === "Home") next = tabs[0];
  else if (event.key === "End") next = tabs.at(-1);
  if (!next) return;
  event.preventDefault();
  select(next, true);
});
