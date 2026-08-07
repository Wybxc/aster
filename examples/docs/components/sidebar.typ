#import "/lib.typ": doc-url, docs-by-section

#let sidebar(current) = [
  #metadata(
    ```css
    .sidebar-backdrop {
      display: none;
    }

    .docs-sidebar {
      position: fixed;
      z-index: 10;
      inset: var(--sl-nav-height) auto 0 0;
      width: var(--sl-sidebar-width);
      overflow-y: auto;
      border-right: 1px solid var(--sl-color-hairline);
      padding: 1.5rem 1rem 2rem;
      background: var(--sl-color-bg-sidebar);
      scrollbar-gutter: stable;
    }

    .docs-sidebar nav {
      display: grid;
      gap: 1.4rem;
    }

    .sidebar-group h2 {
      margin: 0 0 0.35rem;
      padding-inline: 0.5rem;
      color: var(--sl-color-white);
      font-size: 0.875rem;
      font-weight: 600;
      line-height: 1.3;
    }

    .sidebar-group ul {
      display: grid;
      gap: 0.1rem;
      padding: 0;
      list-style: none;
    }

    .sidebar-group a {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.5rem;
      border-radius: 0.35rem;
      padding: 0.3rem 0.5rem;
      color: var(--sl-color-gray-2);
      font-size: 0.875rem;
      line-height: 1.5;
      text-decoration: none;
    }

    .sidebar-group a:hover {
      color: var(--sl-color-white);
    }

    .sidebar-group a[aria-current="page"] {
      background: var(--sl-color-accent-low);
      color: var(--sl-color-text-accent);
      font-weight: 600;
    }

    .sidebar-badge {
      flex: none;
      border: 1px solid var(--sl-color-gray-5);
      border-radius: 0.75rem;
      padding: 0.05rem 0.4rem;
      color: var(--sl-color-gray-3);
      font-size: 0.6875rem;
      font-weight: 600;
      line-height: 1.4;
    }

    @media (max-width: 49.99rem) {
      .sidebar-backdrop {
        position: fixed;
        z-index: 8;
        inset: var(--sl-nav-height) 0 0;
        background: hsl(224 13% 10% / 0.66);
      }

      .docs-sidebar {
        z-index: 9;
        width: min(19rem, calc(100% - 3rem));
        border-right-color: var(--sl-color-gray-5);
        translate: -100% 0;
        transition: translate 180ms ease;
      }

      [data-sidebar-open="true"] .sidebar-backdrop {
        display: block;
      }

      [data-sidebar-open="true"] .docs-sidebar {
        translate: 0 0;
      }
    }
    ```
  ) <aster-style>
  #html.div(class: "sidebar-backdrop print-hidden", id: "sidebar-backdrop")
  #html.aside(class: "docs-sidebar print-hidden", id: "docs-sidebar")[
    #html.nav(aria-label: "Main navigation")[
      #for section in docs-by-section() {
        html.section(class: "sidebar-group")[
          #html.h2[#section.label]
          #html.ul[
            #for item in section.docs {
              let meta = item.metadata
              html.li[
                #if item.id == current {
                  html.a(href: doc-url(item.id), aria-current: "page")[
                    #html.span[#meta.title]
                    #if meta.at("badge", default: none) != none {
                      html.span(class: "sidebar-badge")[#meta.badge]
                    }
                  ]
                } else {
                  html.a(href: doc-url(item.id))[
                    #html.span[#meta.title]
                    #if meta.at("badge", default: none) != none {
                      html.span(class: "sidebar-badge")[#meta.badge]
                    }
                  ]
                }
              ]
            }
          ]
        ]
      }
    ]
  ]
]
