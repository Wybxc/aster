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
})();
