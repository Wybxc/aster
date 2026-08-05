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
  #metadata("./header.css") <aster-style>
  #metadata("./header.js") <aster-module>
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
