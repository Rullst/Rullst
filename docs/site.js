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

const revealTargets = [...document.querySelectorAll("[data-reveal]")];
const revealAll = () => revealTargets.forEach((target) => target.classList.add("is-visible"));
const motionPreference = window.matchMedia("(prefers-reduced-motion: reduce)");
const reducedMotion = motionPreference.matches;

// One bounded intro, not a permanent render loop. Never persist preferences,
// autoplay again on tab return, or override the OS reduced-motion setting.
const introButton = document.querySelector("[data-intro-toggle]");
let introTimer;
const stopIntro = () => {
  clearTimeout(introTimer);
  document.documentElement.classList.remove("intro-playing");
  if (introButton) {
    introButton.textContent = "Replay intro";
    introButton.setAttribute("aria-pressed", "false");
  }
};
const playIntro = () => {
  if (motionPreference.matches || document.hidden || !introButton) return;
  stopIntro();
  document.documentElement.classList.add("intro-playing");
  introButton.textContent = "Stop intro";
  introButton.setAttribute("aria-pressed", "true");
  introTimer = setTimeout(stopIntro, 4800);
};
if (introButton) {
  introButton.hidden = reducedMotion;
  introButton.addEventListener("click", () => {
    if (document.documentElement.classList.contains("intro-playing")) stopIntro();
    else playIntro();
  });
  motionPreference.addEventListener("change", () => {
    introButton.hidden = motionPreference.matches;
    stopIntro();
    if (motionPreference.matches) revealAll();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) stopIntro();
  });
  window.addEventListener("pagehide", stopIntro);
  playIntro();
}

if (reducedMotion || !("IntersectionObserver" in window)) {
  revealAll();
} else {
  const revealObserver = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) return;
      entry.target.classList.add("is-visible");
      revealObserver.unobserve(entry.target);
    });
  }, { rootMargin: "0px 0px -8%", threshold: 0.08 });
  revealTargets.forEach((target) => revealObserver.observe(target));
}
