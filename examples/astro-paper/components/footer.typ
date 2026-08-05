#import "/lib.typ": settings
#import "social-links.typ": social-links

#let footer() = [
  #metadata("./footer.css") <aster-style>
  #html.elem("footer")[
    #html.elem("div")[
      #social-links()
      #html.elem("p")[Copyright #datetime.today().display("[year]") #settings.site.author. All rights reserved.]
    ]
  ]
]
