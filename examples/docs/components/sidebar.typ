#import "/lib.typ": doc-url, docs-by-section

#let sidebar-item-label(node) = [
  #html.span[#node.label]
  #if node.badge != none {
    html.span(class: "sidebar-badge")[#node.badge]
  }
]

#let sidebar-link(node, current) = {
  if node.id == current {
    html.a(href: node.href, aria-current: "page")[#sidebar-item-label(node)]
  } else if current.starts-with(node.id + "/") {
    html.a(href: node.href, class: "sidebar-ancestor")[#sidebar-item-label(node)]
  } else {
    html.a(href: node.href)[#sidebar-item-label(node)]
  }
}

#let sidebar-list(nodes, current) = {
  html.ul[
    #for node in nodes {
      html.li[
        #if node.href != none {
          sidebar-link(node, current)
        } else {
          html.span(class: "sidebar-folder")[#node.label]
        }
        #if node.children.len() > 0 {
          sidebar-list(node.children, current)
        }
      ]
    }
  ]
}

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

    .sidebar-group li > ul {
      margin-top: 0.1rem;
      padding-inline-start: 0.75rem;
      border-inline-start: 1px solid var(--sl-color-hairline);
    }

    .sidebar-group li > ul a {
      padding-block: 0.22rem;
      font-size: 0.8125rem;
    }

    .sidebar-group a.sidebar-ancestor {
      color: var(--sl-color-white);
    }

    .sidebar-folder {
      padding: 0.3rem 0.5rem;
      color: var(--sl-color-gray-3);
      font-size: 0.8125rem;
      font-weight: 600;
      line-height: 1.5;
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
          #sidebar-list(section.children, current)
        ]
      }
    ]
  ]
]
