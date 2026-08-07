#import "/lib.typ": settings
#import "icons.typ": archive-icon, close-icon, menu-icon, moon-icon, sun-icon

#let _nav-link(label, href, active, key) = html.li[
  #if active == key {
    html.a(class: "menu-link", href: href, aria-current: "page")[#label]
  } else {
    html.a(class: "menu-link", href: href)[#label]
  }
]

#let header(active: "") = [
  #metadata(
    ```css
    #skip-to-content {
      position: fixed;
      inset-inline-start: 1rem;
      top: 0;
      z-index: 50;
      translate: 0 -100%;
      background-color: var(--background);
      padding: 0.5rem 0.75rem;
      color: var(--accent);
      transition: translate 150ms cubic-bezier(0.4, 0, 0.2, 1);
    }

    #skip-to-content:focus {
      translate: 0 1rem;
    }

    .skip-links {
      position: absolute;
    }

    .site-header {
      width: 100%;
      max-width: var(--content-width);
      margin-inline: auto;
      padding-inline: 1rem;
    }

    .site-header > div {
      position: relative;
      display: flex;
      min-height: 5rem;
      align-items: center;
      justify-content: space-between;
      border-bottom-width: 1px;
    }

    .site-header > div > a {
      padding-block: 0.25rem;
      font-size: 1.5rem;
      font-weight: 600;
      line-height: 2rem;
      text-decoration: none;
    }

    .site-header > div > nav,
    #menu-items {
      display: flex;
      align-items: center;
    }

    #menu-items {
      gap: 0.75rem;
    }

    .menu-link {
      display: block;
      padding: 0.5rem;
      font-weight: 500;
      text-decoration: none;
    }

    #menu-items [aria-current="page"] {
      color: var(--accent);
      text-decoration: underline wavy 2px;
      text-underline-offset: 8px;
    }

    .icon-button {
      position: relative;
      display: inline-flex;
      width: 2.5rem;
      height: 2.5rem;
      flex-shrink: 0;
      align-items: center;
      justify-content: center;
      background-color: transparent;
      padding: 0.5rem;
      color: var(--foreground);
      text-decoration: none;
    }

    .icon-button:hover {
      color: var(--accent);
    }

    #menu-button,
    .menu-close-icon,
    .theme-sun-icon,
    [data-theme="dark"] .theme-moon-icon {
      display: none;
    }

    [data-theme="dark"] .theme-sun-icon {
      display: block;
    }

    @media (max-width: 639px) {
      .site-header > div {
        min-height: 4rem;
        align-items: flex-start;
        padding-block: 0.75rem;
      }

      .site-header > div > a {
        padding-block: 0.5rem;
        font-size: 1.25rem;
        line-height: 1.75rem;
      }

      .site-header > div > nav {
        flex-direction: column;
        align-items: flex-end;
      }

      #menu-button {
        display: inline-flex;
      }

      #menu-items {
        display: none;
        width: 11rem;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.5rem;
        padding-top: 0.75rem;
      }

      .site-header > div > nav:has(#menu-button[aria-expanded="true"]) #menu-items {
        display: grid;
      }

      #menu-items li {
        grid-column: span 2 / span 2;
      }

      .icon-item {
        grid-column: span 1 / span 1;
        display: flex;
        justify-content: center;
      }

      .menu-link {
        text-align: center;
      }

      #menu-button[aria-expanded="true"] .menu-open-icon {
        display: none;
      }

      #menu-button[aria-expanded="true"] .menu-close-icon {
        display: block;
      }
    }
    ```
  ) <aster-style>
  #metadata(
    ```js
    (() => {
      const root = document.documentElement;
      const themeButton = document.querySelector("#theme-button");
      const menuButton = document.querySelector("#menu-button");

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
      });
    })();
    ```
  ) <aster-script>
  #html.nav(class: "skip-links", aria-label: "Skip links")[
    #html.a(id: "skip-to-content", href: "#main-content")[Skip to content]
  ]
  #html.header(class: "site-header")[
    #html.div[
      #html.a(href: "/")[#settings.site.title]
      #html.nav(aria-label: "Primary navigation")[
        #html.button(
          class: "icon-button",
          id: "menu-button",
          type: "button",
          aria-label: "Open menu",
          aria-expanded: false,
          aria-controls: "menu-items",
        )[
          #html.span(class: "menu-open-icon")[#menu-icon]
          #html.span(class: "menu-close-icon")[#close-icon]
        ]
        #html.ul(id: "menu-items")[
          #_nav-link("Posts", "/posts/", active, "posts")
          #_nav-link("Tags", "/tags/", active, "tags")
          #_nav-link("About", "/about/", active, "about")
          #html.li(class: "icon-item")[
            #if active == "archives" {
              html.a(
                class: "icon-button",
                href: "/archives/",
                title: "Archives",
                aria-label: "Archives",
                aria-current: "page",
              )[#archive-icon]
            } else {
              html.a(
                class: "icon-button",
                href: "/archives/",
                title: "Archives",
                aria-label: "Archives",
              )[#archive-icon]
            }
          ]
          #if settings.features.theme-toggle {
            html.li(class: "icon-item")[
              #html.button(
                class: "icon-button",
                id: "theme-button",
                type: "button",
                title: "Toggle theme",
                aria-label: "Toggle theme",
              )[
                #html.span(class: "theme-moon-icon")[#moon-icon]
                #html.span(class: "theme-sun-icon")[#sun-icon]
              ]
            ]
          }
        ]
      ]
    ]
  ]
]
