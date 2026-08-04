#import "/lib.typ": settings
#import "social-links.typ": social-links

#let footer() = html.elem("footer", attrs: (class: "site-footer"))[
  #html.elem("div", attrs: (class: "footer-inner"))[
    #social-links()
    #html.elem("p")[Copyright #datetime.today().display("[year]") #settings.site.author. All rights reserved.]
  ]
]
