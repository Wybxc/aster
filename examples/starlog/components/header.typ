#import "/lib.typ": settings

#let header() = html.header[
  #html.nav[
    #html.h2(id: "site-title")[
      #html.a(href: "/")[
        #html.span(class: "logo", aria-hidden: true)[#("*")]
        #settings.site.title
      ]
    ]
    #html.a(href: "mailto:contactus@yourwebsite.example")[Contact]
  ]
]
