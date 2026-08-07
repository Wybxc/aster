#let _toc-list(items, class: "") = html.ul(class: class)[
  #for item in items {
    html.li(class: "toc-level-" + str(item.at("level", default: 2)))[
      #html.a(href: "#" + item.id)[#item.title]
    ]
  }
]

#let table-of-contents(items) = [
  #metadata(
    ```css
    .desktop-toc {
      position: sticky;
      top: calc(var(--sl-nav-height) + 1rem);
      align-self: start;
      width: var(--sl-toc-width);
      max-height: calc(100vh - var(--sl-nav-height) - 2rem);
      overflow-y: auto;
      padding: 0.25rem 0 1rem 1rem;
    }

    .desktop-toc h2 {
      margin-bottom: 0.5rem;
      color: var(--sl-color-white);
      font-size: 0.875rem;
      font-weight: 600;
    }

    .desktop-toc ul,
    .mobile-toc ul {
      display: grid;
      gap: 0.35rem;
      padding: 0;
      list-style: none;
    }

    .desktop-toc a,
    .mobile-toc a {
      display: block;
      color: var(--sl-color-gray-3);
      font-size: 0.8125rem;
      line-height: 1.45;
      text-decoration: none;
      overflow-wrap: anywhere;
    }

    .desktop-toc a:hover,
    .desktop-toc a[aria-current="true"],
    .mobile-toc a:hover,
    .mobile-toc a[aria-current="true"] {
      color: var(--sl-color-white);
    }

    .toc-level-3 {
      padding-inline-start: 0.75rem;
    }

    .mobile-toc {
      display: none;
    }

    @media (max-width: 71.99rem) {
      .desktop-toc {
        display: none;
      }

      .mobile-toc {
        display: block;
        position: sticky;
        z-index: 5;
        top: var(--sl-nav-height);
        margin-inline: calc(-1 * var(--sl-content-pad-x));
        border-bottom: 1px solid var(--sl-color-hairline);
        background: var(--sl-color-bg-nav);
      }

      .mobile-toc summary {
        display: flex;
        min-height: 3rem;
        align-items: center;
        gap: 0.5rem;
        padding: 0.6rem var(--sl-content-pad-x);
        color: var(--sl-color-white);
        font-size: 0.8125rem;
        cursor: pointer;
        list-style: none;
      }

      .mobile-toc summary::-webkit-details-marker {
        display: none;
      }

      .mobile-toc summary::after {
        content: ">";
        margin-left: auto;
        color: var(--sl-color-gray-3);
        rotate: 90deg;
      }

      .mobile-toc details[open] summary::after {
        rotate: -90deg;
      }

      .mobile-toc ul {
        max-height: 55vh;
        overflow-y: auto;
        border-top: 1px solid var(--sl-color-hairline);
        padding: 0.75rem var(--sl-content-pad-x) 1rem;
        background: var(--sl-color-bg);
      }
    }
    ```
  ) <aster-style>
  #metadata(
    ```js
    (() => {
      const links = [...document.querySelectorAll('.page-toc a[href^="#"]')];
      const targets = links
        .map((link) => document.getElementById(link.hash.slice(1)))
        .filter(Boolean);
      if (!targets.length) return;

      const setCurrent = (id) => {
        links.forEach((link) => {
          if (link.hash === `#${id}`) link.setAttribute("aria-current", "true");
          else link.removeAttribute("aria-current");
        });
      };
      const observer = new IntersectionObserver((entries) => {
        const visible = entries.filter((entry) => entry.isIntersecting).at(-1);
        if (visible) setCurrent(visible.target.id);
      }, { rootMargin: "-20% 0px -65% 0px" });
      targets.forEach((target) => observer.observe(target));
      setCurrent(targets[0].id);

      document.querySelectorAll(".mobile-toc a").forEach((link) => {
        link.addEventListener("click", () => link.closest("details")?.removeAttribute("open"));
      });
    })();
    ```
  ) <aster-script>
  #html.nav(class: "desktop-toc page-toc print-hidden", aria-labelledby: "on-this-page")[
    #html.h2(id: "on-this-page")[On this page]
    #_toc-list(items)
  ]
]

#let mobile-table-of-contents(items) = html.nav(
  class: "mobile-toc page-toc print-hidden",
  aria-label: "On this page",
)[
  #html.elem("details")[
    #html.elem("summary")[On this page]
    #_toc-list(items)
  ]
]
