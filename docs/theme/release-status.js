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
    title.textContent = "⚠️ Release status: v12.0.0-rc.1 candidate / v5 legacy";

    const mainLink = document.createElement("a");
    mainLink.href = "https://github.com/Rullst/Rullst/tree/main";
    mainLink.textContent = "main";

    const v5Link = document.createElement("a");
    v5Link.href = "https://github.com/Rullst/Rullst/tree/v5";
    v5Link.textContent = "v5";

    const message = document.createElement("span");
    message.append(
      "The ",
      mainLink,
      " branch contains active v12 work. The v12.0.0-rc.1 candidate passed its " +
        "documented release gates and is approved for explicit prerelease evaluation; " +
        "it is not the stable v12 release. The frozen ",
      v5Link,
      " branch preserves legacy source without ongoing maintenance. Use exact " +
        "release artifacts rather than the moving main branch."
    );

    banner.append(title, message);

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
