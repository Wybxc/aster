#import "/lib.typ": settings

#let navigation(root) = html.elem("header")[
  #html.elem("nav", attrs: ("aria-label": "Primary navigation"))[
    #link(root + "index.html")[*#settings.site.title*]
    #for item in settings.navigation {
      link(root + item.href)[#item.label]
    }
  ]
]
