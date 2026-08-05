#import "/lib.typ": settings

#let navigation() = html.elem("header")[
  #html.elem("nav", attrs: ("aria-label": "Primary navigation"))[
    #link("/")[*#settings.site.title*]
    #for item in settings.navigation {
      link(item.href)[#item.label]
    }
  ]
]
