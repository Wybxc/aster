(() => {
  const stored = localStorage.getItem("aster-docs-theme") || "auto";
  const system = matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  document.documentElement.dataset.themeChoice = stored;
  document.documentElement.dataset.theme = stored === "auto" ? system : stored;
})();
