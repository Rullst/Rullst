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
    title.textContent = "Rullst v12";

    const stableLink = document.createElement("a");
    stableLink.href = "https://github.com/Rullst/Rullst/tree/v12";
    stableLink.textContent = "v12";

    const mainLink = document.createElement("a");
    mainLink.href = "https://github.com/Rullst/Rullst/tree/main";
    mainLink.textContent = "main";

    const message = document.createElement("span");
    message.append(
      "Build with published v12 packages. Stable maintenance is on ", stableLink,
      "; next-major development is on ", mainLink, "."
    );

    const details = document.createElement("details");
    const summary = document.createElement("summary");
    summary.textContent = "Release reproducibility & legacy versions";
    const legacy = document.createElement("p");
    legacy.textContent =
      "Use the exact crates.io version and matching immutable tag to reproduce a published release. " +
      "A source branch or development documentation does not establish publication or security support.";
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
