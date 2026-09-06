document.documentElement.classList.add("js");

const toggle = document.querySelector("[data-nav-toggle]");
const navigation = document.querySelector("[data-navigation]");

if (toggle && navigation) {
  const closeNavigation = () => {
    toggle.setAttribute("aria-expanded", "false");
    navigation.classList.remove("is-open");
  };
  toggle.addEventListener("click", () => {
    const open = toggle.getAttribute("aria-expanded") === "true";
    toggle.setAttribute("aria-expanded", String(!open));
    navigation.classList.toggle("is-open", !open);
  });
  navigation.querySelectorAll("a").forEach((link) => {
    link.addEventListener("click", closeNavigation);
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && toggle.getAttribute("aria-expanded") === "true") {
      closeNavigation();
      toggle.focus();
    }
  });
}

const copyStatus = document.querySelector("[data-copy-status]");
document.querySelectorAll("[data-copy-command]").forEach((button) => {
  const command = document.getElementById(button.dataset.copyCommand);
  if (!command) return;
  button.addEventListener("click", async () => {
    let message;
    try {
      await navigator.clipboard.writeText(command.textContent.trim());
      message = "Command copied.";
      button.textContent = "Copied";
    } catch (_error) {
      message = "Clipboard unavailable. Select and copy the displayed command.";
      button.textContent = "Select text";
    }
    if (copyStatus) copyStatus.textContent = message;
  });
});
