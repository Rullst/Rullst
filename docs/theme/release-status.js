(() => {
  const renderReleaseStatus = () => {
    if (document.getElementById("rullst-release-status")) {
      return;
    }

    const banner = document.createElement("aside");
    banner.id = "rullst-release-status";
    banner.className = "rullst-release-status";
    banner.setAttribute("aria-label", "Rullst release status");

    const title = document.createElement("strong");
    title.className = "rullst-release-status__title";
    title.textContent = "Rullst v12.0.0 stable";

    const mainLink = document.createElement("a");
    mainLink.href = "https://github.com/Rullst/Rullst/tree/main";
    mainLink.textContent = "main";

    const v5Link = document.createElement("a");
    v5Link.href = "https://github.com/Rullst/Rullst/tree/v5";
    v5Link.textContent = "v5";

    const message = document.createElement("span");
    message.append(
      "Build with published v12 packages. The ",
      mainLink,
      " branch tracks maintenance; new major-version development is on v13."
    );

    const details = document.createElement("details");
    const summary = document.createElement("summary");
    summary.textContent = "Release reproducibility & legacy versions";
    const legacy = document.createElement("p");
    legacy.append(
      "Use the exact v12.0.0 crates.io packages or immutable tag to reproduce " +
        "the release, rather than a moving branch. The frozen ",
      v5Link,
      " branch preserves historical source without ongoing maintenance."
    );
    details.append(summary, legacy);
    banner.append(title, message, details);

    const bookContent = document.getElementById("mdbook-content");
    if (bookContent) {
      bookContent.prepend(banner);
    } else {
      document.body.prepend(banner);
    }
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", renderReleaseStatus, { once: true });
  } else {
    renderReleaseStatus();
  }
})();
