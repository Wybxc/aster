(() => {
  const root = document.documentElement;
  const themeButton = document.querySelector("#theme-button");
  const menuButton = document.querySelector("#menu-button");
  const menuItems = document.querySelector("#menu-items");

  const reflectTheme = () => {
    const theme = root.dataset.theme === "dark" ? "dark" : "light";
    themeButton?.setAttribute("aria-label", `Use ${theme === "dark" ? "light" : "dark"} theme`);
    const background = getComputedStyle(document.body).backgroundColor;
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", background);
  };

  themeButton?.addEventListener("click", () => {
    const theme = root.dataset.theme === "dark" ? "light" : "dark";
    root.dataset.theme = theme;
    try {
      localStorage.setItem("theme", theme);
    } catch (_) {
      // The selected theme still applies for the current document.
    }
    reflectTheme();
  });
  reflectTheme();

  menuButton?.addEventListener("click", () => {
    const open = menuButton.getAttribute("aria-expanded") === "true";
    menuButton.setAttribute("aria-expanded", String(!open));
    menuButton.setAttribute("aria-label", open ? "Open menu" : "Close menu");
    menuItems?.classList.toggle("open", !open);
  });

  const slugify = value =>
    value
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)/g, "");

  const usedIds = new Set();
  document.querySelectorAll("#article h2, #article h3, #article h4, #article h5, #article h6").forEach(heading => {
    let id = heading.id || slugify(heading.textContent || "section") || "section";
    const base = id;
    let suffix = 2;
    while (usedIds.has(id) || document.querySelector(`[id="${CSS.escape(id)}"]`)) {
      id = `${base}-${suffix++}`;
    }
    heading.id = id;
    usedIds.add(id);

    const anchor = document.createElement("a");
    anchor.className = "heading-link";
    anchor.href = `#${id}`;
    anchor.setAttribute("aria-label", `Link to ${heading.textContent}`);
    anchor.textContent = "#";
    heading.append(anchor);
  });

  document.querySelectorAll("#article pre").forEach(block => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "copy-code";
    button.textContent = "Copy";
    button.addEventListener("click", async () => {
      const text = block.querySelector("code")?.textContent || "";
      await navigator.clipboard.writeText(text);
      button.textContent = "Copied";
      setTimeout(() => {
        button.textContent = "Copy";
      }, 900);
    });
    block.append(button);
  });

  const progress = document.querySelector("#reading-progress");
  const backToTop = document.querySelector("#back-to-top");
  const updateScroll = () => {
    const height = root.scrollHeight - root.clientHeight;
    const ratio = height > 0 ? root.scrollTop / height : 0;
    if (progress) progress.style.width = `${Math.min(100, ratio * 100)}%`;
    backToTop?.classList.toggle("visible", root.scrollTop > 480);
  };
  document.addEventListener("scroll", updateScroll, { passive: true });
  updateScroll();
  backToTop?.addEventListener("click", () => scrollTo({ top: 0, behavior: "smooth" }));

  const article = document.querySelector("#article");
  article?.addEventListener("click", event => {
    const image = event.target.closest("img");
    if (!image || image.closest("a")) return;

    const dialog = document.createElement("div");
    dialog.className = "lightbox";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", image.alt ? `Image preview: ${image.alt}` : "Image preview");

    const preview = image.cloneNode();
    preview.alt = "";
    const close = document.createElement("button");
    close.type = "button";
    close.className = "lightbox-close";
    close.title = "Close image preview";
    close.setAttribute("aria-label", "Close image preview");
    close.textContent = "×";

    const dismiss = () => {
      dialog.remove();
      document.body.style.overflow = "";
      image.focus();
    };
    close.addEventListener("click", dismiss);
    dialog.addEventListener("click", event => {
      if (event.target === dialog) dismiss();
    });
    dialog.addEventListener("keydown", event => {
      if (event.key === "Escape") dismiss();
    });

    image.tabIndex = 0;
    dialog.append(preview, close);
    document.body.append(dialog);
    document.body.style.overflow = "hidden";
    close.focus();
  });
})();
