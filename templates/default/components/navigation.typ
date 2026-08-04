#import "/lib.typ": settings

#let navigation() = html.elem("header")[
  #html.elem("nav")[#link("/")[*#settings.site.title*]]
]
