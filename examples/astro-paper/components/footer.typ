#import "/lib.typ": settings
#import "social-links.typ": social-links

#let footer() = [
  #metadata("./footer.css") <aster-style>
  #html.footer(class: "site-footer")[
    #html.div[
      #social-links()

      Copyright #datetime.today().display("[year]") #settings.site.author. All rights reserved.
    ]
  ]
]
