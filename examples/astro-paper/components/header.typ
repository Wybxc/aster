#import "/lib.typ": settings
#import "icons.typ": archive-icon, close-icon, menu-icon, moon-icon, sun-icon

#let _nav-link(label, href, active, key) = html.elem("li")[
  #html.elem("a", attrs: (
    href: href,
    class: if active == key { "nav-link active" } else { "nav-link" },
  ))[#label]
]

#let header(active: "") = [
  #html.elem("div", attrs: (class: "skip-link-host"))[
    #html.elem("a", attrs: (
      id: "skip-to-content",
      class: "skip-link",
      href: "#main-content",
    ))[Skip to content]
  ]
  #html.elem("header", attrs: (class: "site-header"))[
    #html.elem("div", attrs: (class: "header-inner"))[
      #html.elem("a", attrs: (class: "brand", href: "/"))[
        #settings.site.title
      ]
      #html.elem("nav", attrs: (
        class: "site-nav",
        "aria-label": "Primary navigation",
      ))[
        #html.elem("button", attrs: (
          id: "menu-button",
          class: "icon-button menu-button",
          type: "button",
          "aria-label": "Open menu",
          "aria-expanded": "false",
          "aria-controls": "menu-items",
        ))[
          #html.elem("span", attrs: (class: "menu-open-icon"))[#menu-icon]
          #html.elem("span", attrs: (class: "menu-close-icon"))[#close-icon]
        ]
        #html.elem("ul", attrs: (id: "menu-items", class: "nav-items"))[
          #_nav-link("Posts", "/posts/", active, "posts")
          #_nav-link("Tags", "/tags/", active, "tags")
          #_nav-link("About", "/about/", active, "about")
          #html.elem("li")[
            #html.elem("a", attrs: (
              href: "/archives/",
              class: if active == "archives" { "icon-button active" } else { "icon-button" },
              title: "Archives",
              "aria-label": "Archives",
            ))[#archive-icon]
          ]
          #if settings.features.theme-toggle {
            html.elem("li")[
              #html.elem("button", attrs: (
                id: "theme-button",
                class: "icon-button theme-button",
                type: "button",
                title: "Toggle theme",
                "aria-label": "Toggle theme",
              ))[
                #html.elem("span", attrs: (class: "moon-icon"))[#moon-icon]
                #html.elem("span", attrs: (class: "sun-icon"))[#sun-icon]
              ]
            ]
          }
        ]
      ]
    ]
  ]
]
