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

    const devLink = document.createElement("a");
    devLink.href = "https://github.com/Rullst/Rullst/tree/dev";
    devLink.textContent = "dev";

    const message = document.createElement("span");
    message.append(
      "The ",
      mainLink,
      " branch is the end-of-life v5 baseline and no longer receives maintenance. " +
        "Active v12 work lives on ",
      devLink,
      ", is unreleased, and remains NO-GO for production until its documented " +
        "release gates pass."
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
