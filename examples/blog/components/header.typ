#import "/lib.typ": settings

#let nav-link(name, href, active) = html.a(
  href: href,
  class: if active { "active" } else { "" },
)[#name]

#let header(active: "") = [
  #metadata(
    ```css
    .site-header {
      padding: 0 1rem;
      background: white;
      box-shadow: 0 2px 8px rgb(96 115 159 / 18%);
    }

    .site-header nav {
      display: flex;
      max-width: 72rem;
      min-height: 4.5rem;
      margin: auto;
      align-items: center;
      gap: 1.25rem;
    }

    .site-header strong {
      margin-right: auto;
      color: rgb(var(--black));
      font-size: 1.15rem;
    }

    .site-header a {
      border-bottom: 4px solid transparent;
      padding: 1.35rem 0.25rem 1.1rem;
      color: rgb(var(--gray));
      text-decoration: none;
    }

    .site-header a:hover,
    .site-header a.active {
      border-bottom-color: var(--accent);
      color: rgb(var(--black));
    }

    .site-header .social {
      display: flex;
      gap: 0.75rem;
      margin-left: 0.5rem;
    }

    @media (max-width: 40rem) {
      .site-header nav { gap: 0.75rem; }
      .site-header strong { font-size: 1rem; }
      .site-header .social { display: none; }
    }
    ```
  ) <aster-style>
  #html.header(class: "site-header")[
    #html.nav(aria-label: "Primary navigation")[
      #html.strong[#settings.site.title]
      #nav-link("Home", "/", active == "home")
      #nav-link("Blog", "/blog/", active == "blog")
      #nav-link("About", "/about/", active == "about")
      #html.span(class: "social")[
        #html.a(href: "https://github.com/Wybxc/aster", aria-label: "Aster source code")[Code]
        #html.a(href: "mailto:hello@example.com", aria-label: "Email the editors")[Mail]
      ]
    ]
  ]
]
