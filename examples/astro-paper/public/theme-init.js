(() => {
  let theme = null;
  try {
    theme = localStorage.getItem("theme");
  } catch (_) {
    // Storage can be disabled without preventing the page from rendering.
  }
  if (theme !== "light" && theme !== "dark") {
    theme = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  document.documentElement.dataset.theme = theme;
})();
