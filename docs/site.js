document.documentElement.classList.add("js");

// Interface text: English unless the page translates it with data-ui-*
// attributes on <body> (the Portuguese landing, site/src/pages/landing/pt_top.rs, does).
const ui = document.body.dataset;
const text = {
  copied: ui.uiCopied ?? "Command copied.",
  copiedButton: ui.uiCopiedButton ?? "Copied",
  copyFailed: ui.uiCopyFailed ?? "Clipboard unavailable. Select and copy the displayed command.",
  copyFailedButton: ui.uiCopyFailedButton ?? "Select text",
  replayIntro: ui.uiReplayIntro ?? "Replay intro",
  stopIntro: ui.uiStopIntro ?? "Stop intro",
  sayHello: ui.uiSayHello ?? "Say hello ↗",
  stopWaving: ui.uiStopWaving ?? "Stop waving",
};

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
      message = text.copied;
      button.textContent = text.copiedButton;
    } catch (_error) {
      message = text.copyFailed;
      button.textContent = text.copyFailedButton;
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
    introButton.textContent = text.replayIntro;
    introButton.setAttribute("aria-pressed", "false");
  }
};
const playIntro = () => {
  if (motionPreference.matches || document.hidden || !introButton) return;
  stopIntro();
  document.documentElement.classList.add("intro-playing");
  introButton.textContent = text.stopIntro;
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

// Greet once when the Academy comes into view. Further waves are requested
// explicitly; motion stays bounded and never overrides the OS preference.
const academy = document.querySelector("#academy");
const academyWaveButton = document.querySelector("[data-academy-wave]");
if (academy && academyWaveButton) {
  let academyWaveTimer;
  const stopAcademyWave = () => {
    clearTimeout(academyWaveTimer);
    academy.classList.remove("academy-is-waving");
    academyWaveButton.textContent = text.sayHello;
    academyWaveButton.setAttribute("aria-pressed", "false");
  };
  const playAcademyWave = () => {
    if (motionPreference.matches || document.hidden || academy.classList.contains("academy-is-waving")) return;
    academy.classList.add("academy-is-waving");
    academyWaveButton.textContent = text.stopWaving;
    academyWaveButton.setAttribute("aria-pressed", "true");
    academyWaveTimer = setTimeout(stopAcademyWave, 4400);
  };
  academyWaveButton.hidden = motionPreference.matches;
  academyWaveButton.addEventListener("click", () => {
    if (academy.classList.contains("academy-is-waving")) stopAcademyWave();
    else playAcademyWave();
  });
  motionPreference.addEventListener("change", () => {
    academyWaveButton.hidden = motionPreference.matches;
    stopAcademyWave();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) stopAcademyWave();
  });
  window.addEventListener("pagehide", stopAcademyWave);
  if ("IntersectionObserver" in window) {
    let academyGreeted = false;
    const academyObserver = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) {
          stopAcademyWave();
        } else if (!academyGreeted) {
          academyGreeted = true;
          playAcademyWave();
        }
      });
    }, { threshold: 0.25 });
    // Observe the mascot itself so the greeting is still visible on tall
    // mobile layouts, where the copy reaches the viewport first.
    academyObserver.observe(academy.querySelector(".academy-companion"));
  }
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
