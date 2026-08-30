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
    title.textContent = "⚠️ Release status: v5 legacy / v12 development preview";

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
      " branch contains active v12 work, is unreleased, and remains NO-GO for " +
        "production until its documented release gates pass. The frozen ",
      v5Link,
      " branch preserves legacy source without ongoing maintenance."
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
