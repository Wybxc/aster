#import "/lib.typ": settings

#let link-item(label, href, active, key) = html.li[
  #html.a(
    href: href,
    class: "nav-link",
    aria-current: if active == key { "page" } else { false },
  )[#label]
]

#let nav(active: "") = [
  #metadata(
    ```js
    (() => {
      const root = document.documentElement;
      const menu = document.querySelector("#site-menu");
      const menuButton = document.querySelector("#menu-button");
      const themeButton = document.querySelector("#theme-button");

      try {
        const saved = localStorage.getItem("portfolio-theme");
        if (saved === "dark") root.classList.add("theme-dark");
      } catch (_) {}

      menuButton?.addEventListener("click", () => {
        const expanded = menuButton.getAttribute("aria-expanded") === "true";
        menuButton.setAttribute("aria-expanded", String(!expanded));
        menu?.classList.toggle("open", !expanded);
      });

      const reflectTheme = () => {
        const dark = root.classList.contains("theme-dark");
        themeButton?.setAttribute("aria-pressed", String(dark));
        themeButton?.setAttribute("aria-label", dark ? "Use light theme" : "Use dark theme");
      };

      themeButton?.addEventListener("click", () => {
        root.classList.toggle("theme-dark");
        try {
          localStorage.setItem("portfolio-theme", root.classList.contains("theme-dark") ? "dark" : "light");
        } catch (_) {}
        reflectTheme();
      });
      reflectTheme();
    })();
    ```
  ) <aster-script>
  #html.nav(class: "site-nav", aria-label: "Primary navigation")[
    #html.a(class: "brand", href: "/")[
      #html.span(class: "brand-mark", aria-hidden: true)[#(">_")]
      #settings.site.title
    ]
    #html.button(id: "menu-button", aria-expanded: false, aria-controls: "site-menu")[Menu]
    #html.div(id: "site-menu")[
      #html.ul[
        #link-item("Home", "/", active, "home")
        #link-item("Work", "/work/", active, "work")
        #link-item("About", "/about/", active, "about")
      ]
      #html.div(class: "nav-actions")[
        #html.a(href: "https://github.com/Wybxc/aster")[Source]
        #html.button(id: "theme-button", aria-pressed: false, aria-label: "Use dark theme")[◐]
      ]
    ]
  ]
]
