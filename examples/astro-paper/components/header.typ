#import "/lib.typ": settings
#import "icons.typ": archive-icon, close-icon, menu-icon, moon-icon, sun-icon

#let _nav-link(label, href, active, key) = {
  let attrs = (href: href)
  if active == key {
    attrs.insert("aria-current", "page")
  }
  html.elem("li")[#html.elem("a", attrs: attrs)[#label]]
}

#let header(active: "") = [
  #metadata("./header.css") <aster-style>
  #metadata("./header.js") <aster-module>
  #html.elem("nav", attrs: ("aria-label": "Skip links"))[
    #html.elem("a", attrs: (id: "skip-to-content", href: "#main-content"))[Skip to content]
  ]
  #html.elem("header")[
    #html.elem("div")[
      #html.elem("a", attrs: (href: "/"))[#settings.site.title]
      #html.elem("nav", attrs: ("aria-label": "Primary navigation"))[
        #html.elem("button", attrs: (
          id: "menu-button",
          type: "button",
          "aria-label": "Open menu",
          "aria-expanded": "false",
          "aria-controls": "menu-items",
        ))[
          #html.elem("span")[#menu-icon]
          #html.elem("span")[#close-icon]
        ]
        #html.elem("ul", attrs: (id: "menu-items"))[
          #_nav-link("Posts", "/posts/", active, "posts")
          #_nav-link("Tags", "/tags/", active, "tags")
          #_nav-link("About", "/about/", active, "about")
          #html.elem("li")[
            #let attrs = (
              href: "/archives/",
              title: "Archives",
              "aria-label": "Archives",
            )
            #if active == "archives" {
              attrs.insert("aria-current", "page")
            }
            #html.elem("a", attrs: attrs)[#archive-icon]
          ]
          #if settings.features.theme-toggle {
            html.elem("li")[
              #html.elem("button", attrs: (
                id: "theme-button",
                type: "button",
                title: "Toggle theme",
                "aria-label": "Toggle theme",
              ))[
                #html.elem("span")[#moon-icon]
                #html.elem("span")[#sun-icon]
              ]
            ]
          }
        ]
      ]
    ]
  ]
]
