#import "/lib.typ": docs, doc-url, settings
#import "icons.typ": close-icon, github-icon, menu-icon, search-icon

#let header() = [
  #metadata(
    ```css
    .site-header {
      position: fixed;
      z-index: 20;
      inset: 0 0 auto;
      height: var(--sl-nav-height);
      border-bottom: 1px solid var(--sl-color-hairline);
      padding: 0.75rem 1.5rem;
      background: var(--sl-color-bg-nav);
    }

    .skip-link {
      position: fixed;
      z-index: 30;
      top: 0;
      left: 1rem;
      translate: 0 -100%;
      border-radius: 0 0 0.375rem 0.375rem;
      padding: 0.5rem 0.75rem;
      background: var(--sl-color-accent);
      color: white;
      font-weight: 600;
      text-decoration: none;
      transition: translate 120ms ease;
    }

    .skip-link:focus {
      translate: 0 0;
    }

    .site-header__inner {
      display: grid;
      grid-template-columns: minmax(15.75rem, auto) minmax(12rem, 28rem) 1fr;
      align-items: center;
      gap: 1.25rem;
      height: 100%;
    }

    .site-title {
      display: inline-flex;
      min-width: 0;
      align-items: center;
      gap: 0.75rem;
      color: var(--sl-color-text-accent);
      font-size: 1.25rem;
      font-weight: 600;
      line-height: 1.2;
      text-decoration: none;
      white-space: nowrap;
    }

    .site-title img {
      width: 2rem;
      height: 2rem;
      flex: none;
      border-radius: 0.45rem;
    }

    .header-actions {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 0.75rem;
    }

    .header-actions > span[style],
    .site-title > span[style] {
      display: none;
    }

    .icon-button,
    .search-button {
      display: inline-flex;
      min-width: 2.25rem;
      height: 2.25rem;
      align-items: center;
      justify-content: center;
      border: 1px solid transparent;
      border-radius: 0.5rem;
      background: transparent;
      color: var(--sl-color-gray-2);
      cursor: pointer;
      text-decoration: none;
    }

    .icon-button:hover,
    .search-button:hover {
      border-color: var(--sl-color-gray-5);
      color: var(--sl-color-white);
    }

    .search-button {
      width: 100%;
      justify-content: flex-start;
      gap: 0.65rem;
      border-color: var(--sl-color-gray-5);
      padding-inline: 0.75rem;
      background: var(--sl-color-black);
      font-size: 0.875rem;
    }

    .search-button kbd {
      margin-inline-start: auto;
      border: 1px solid var(--sl-color-gray-5);
      border-radius: 0.25rem;
      padding: 0.05rem 0.35rem;
      color: var(--sl-color-gray-3);
      font-size: 0.75rem;
    }

    .sl-icon {
      width: 1.25rem;
      height: 1.25rem;
      flex: none;
    }

    .theme-picker {
      height: 2.25rem;
      border: 1px solid var(--sl-color-gray-5);
      border-radius: 0.5rem;
      padding-inline: 0.55rem;
      background: var(--sl-color-black);
      color: var(--sl-color-gray-2);
      font-size: 0.8125rem;
      cursor: pointer;
    }

    #sidebar-toggle,
    .mobile-search,
    .menu-close {
      display: none;
    }

    [data-sidebar-open="true"] .menu-open {
      display: none;
    }

    [data-sidebar-open="true"] .menu-close {
      display: block;
    }

    .search-dialog {
      width: min(38rem, calc(100% - 2rem));
      max-height: min(40rem, calc(100vh - 4rem));
      margin: 8vh auto 0;
      border: 1px solid var(--sl-color-gray-5);
      border-radius: 0.5rem;
      padding: 0;
      background: var(--sl-color-bg-nav);
      color: var(--sl-color-text);
      box-shadow: var(--sl-shadow-md);
    }

    .search-dialog::backdrop {
      background: hsl(224 13% 10% / 0.66);
      backdrop-filter: blur(3px);
    }

    .search-dialog header {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      border-bottom: 1px solid var(--sl-color-gray-5);
      padding: 0.75rem;
    }

    .search-dialog input {
      min-width: 0;
      flex: 1;
      border: 0;
      outline: 0;
      background: transparent;
      color: var(--sl-color-white);
      font-size: 1rem;
    }

    .search-dialog ul {
      display: grid;
      gap: 0.35rem;
      max-height: min(30rem, calc(100vh - 12rem));
      overflow-y: auto;
      padding: 0.75rem;
      list-style: none;
    }

    .search-dialog li[hidden] {
      display: none;
    }

    .search-dialog li a {
      display: block;
      border: 1px solid transparent;
      border-radius: 0.375rem;
      padding: 0.65rem 0.75rem;
      color: var(--sl-color-white);
      text-decoration: none;
    }

    .search-dialog li a:hover,
    .search-dialog li a:focus-visible {
      border-color: var(--sl-color-accent);
      background: var(--sl-color-accent-low);
    }

    .search-dialog li span {
      display: block;
      margin-top: 0.1rem;
      color: var(--sl-color-gray-3);
      font-size: 0.8125rem;
    }

    @media (max-width: 49.99rem) {
      .site-header {
        padding: 0.6rem 1rem;
      }

      .site-header__inner {
        display: flex;
        gap: 0.5rem;
      }

      .site-title {
        margin-right: auto;
        font-size: 1rem;
      }

      .site-title img {
        width: 1.75rem;
        height: 1.75rem;
      }

      .header-search {
        display: none;
      }

      .header-actions {
        gap: 0.25rem;
      }

      #sidebar-toggle {
        display: inline-flex;
      }

      .mobile-search {
        display: inline-flex;
      }

      .theme-picker {
        width: 4.75rem;
        padding-inline: 0.25rem;
      }
    }
    ```
  ) <aster-style>
  #metadata(
    ```js
    (() => {
      const root = document.documentElement;
      const media = matchMedia("(prefers-color-scheme: light)");
      const themePicker = document.querySelector("#theme-picker");
      const sidebarToggle = document.querySelector("#sidebar-toggle");
      const sidebarBackdrop = document.querySelector("#sidebar-backdrop");
      const searchDialog = document.querySelector("#search-dialog");
      const searchInput = document.querySelector("#docs-search");

      const applyTheme = (choice) => {
        const theme = choice === "auto" ? (media.matches ? "light" : "dark") : choice;
        root.dataset.themeChoice = choice;
        root.dataset.theme = theme;
        if (themePicker) themePicker.value = choice;
      };

      const savedTheme = localStorage.getItem("aster-docs-theme") || "auto";
      applyTheme(savedTheme);
      themePicker?.addEventListener("change", () => {
        localStorage.setItem("aster-docs-theme", themePicker.value);
        applyTheme(themePicker.value);
      });
      media.addEventListener("change", () => {
        if (root.dataset.themeChoice === "auto") applyTheme("auto");
      });

      const setSidebar = (open) => {
        root.dataset.sidebarOpen = open ? "true" : "false";
        sidebarToggle?.setAttribute("aria-expanded", String(open));
        sidebarToggle?.setAttribute("aria-label", open ? "Close navigation" : "Open navigation");
      };
      sidebarToggle?.addEventListener("click", () => {
        setSidebar(root.dataset.sidebarOpen !== "true");
      });
      sidebarBackdrop?.addEventListener("click", () => setSidebar(false));
      document.querySelectorAll("#docs-sidebar a").forEach((link) => {
        link.addEventListener("click", () => setSidebar(false));
      });

      const filterSearch = () => {
        const query = searchInput.value.trim().toLowerCase();
        document.querySelectorAll("#search-results li").forEach((item) => {
          item.hidden = query !== "" && !item.dataset.search.includes(query);
        });
      };
      const openSearch = () => {
        searchDialog?.showModal();
        requestAnimationFrame(() => searchInput?.focus());
      };
      document.querySelectorAll("[data-open-search]").forEach((button) => {
        button.addEventListener("click", openSearch);
      });
      searchInput?.addEventListener("input", filterSearch);
      document.addEventListener("keydown", (event) => {
        if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
          event.preventDefault();
          openSearch();
        }
        if (event.key === "Escape") setSidebar(false);
      });

      document.querySelectorAll(".sl-markdown-content pre").forEach((pre) => {
        const button = document.createElement("button");
        button.className = "copy-code";
        button.type = "button";
        button.title = "Copy code";
        button.setAttribute("aria-label", "Copy code");
        button.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
        button.addEventListener("click", async () => {
          const code = pre.querySelector("code")?.textContent || "";
          await navigator.clipboard.writeText(code);
          button.title = "Copied";
          setTimeout(() => { button.title = "Copy code"; }, 1200);
        });
        pre.append(button);
      });
    })();
    ```
  ) <aster-script>
  #html.a(class: "skip-link print-hidden", href: "#main-content")[Skip to content]
  #html.header(class: "site-header print-hidden")[
    #html.div(class: "site-header__inner")[
      #html.a(class: "site-title", href: "/")[
        #html.img(src: "/assets/logo.svg", alt: "", width: 32, height: 32)
        #html.elem("span", attrs: (translate: "no"))[#settings.site.title]
      ]
      #html.div(class: "header-search")[
        #html.elem("button", attrs: (
          class: "search-button",
          type: "button",
          "data-open-search": "",
        ))[
          #search-icon
          #html.span[Search docs]
          #html.elem("kbd")[Ctrl K]
        ]
      ]
      #html.div(class: "header-actions")[
        #html.button(
          class: "icon-button",
          id: "sidebar-toggle",
          type: "button",
          aria-label: "Open navigation",
          aria-expanded: false,
          aria-controls: "docs-sidebar",
        )[
          #html.span(class: "menu-open")[#menu-icon]
          #html.span(class: "menu-close")[#close-icon]
        ]
        #html.elem("button", attrs: (
          class: "icon-button mobile-search",
          type: "button",
          title: "Search",
          "aria-label": "Search documentation",
          "data-open-search": "",
        ))[#search-icon]
        #html.a(
          class: "icon-button",
          href: settings.site.repository,
          title: "GitHub",
          aria-label: "View source on GitHub",
        )[#github-icon]
        #html.elem("select", attrs: (
          class: "theme-picker",
          id: "theme-picker",
          title: "Color theme",
          "aria-label": "Color theme",
        ))[
          #html.elem("option", attrs: (value: "dark"))[Dark]
          #html.elem("option", attrs: (value: "light"))[Light]
          #html.elem("option", attrs: (value: "auto", selected: ""))[Auto]
        ]
      ]
    ]
  ]
  #html.elem("dialog", attrs: (class: "search-dialog", id: "search-dialog"))[
    #html.elem("form", attrs: (method: "dialog"))[
      #html.header[
        #search-icon
        #html.elem("input", attrs: (
          id: "docs-search",
          type: "search",
          placeholder: "Search page titles",
          autocomplete: "off",
          "aria-label": "Search page titles",
        ))
        #html.button(class: "icon-button", type: "submit", aria-label: "Close search")[#close-icon]
      ]
    ]
    #html.ul(id: "search-results")[
      #for item in docs() {
        let meta = item.metadata
        html.elem("li", attrs: ("data-search": lower(meta.title + " " + meta.description)))[
          #html.a(href: doc-url(item.id))[
            #meta.title
            #html.span[#meta.description]
          ]
        ]
      }
    ]
  ]
]
