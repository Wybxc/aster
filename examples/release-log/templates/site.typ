#import "/lib.typ": settings
#import "/components/footer.typ": footer
#import "/components/header.typ": header

#let site(body, title: settings.site.title, description: settings.site.description) = [
  #html.html(lang: settings.site.language)[
    #html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.meta(name: "description", content: description)
      #html.elem("link", attrs: (rel: "icon", href: "/assets/favicon.svg"))
      #html.elem("link", attrs: (rel: "preconnect", href: "https://fonts.googleapis.com"))
      #html.link(rel: "stylesheet", href: "https://fonts.googleapis.com/css2?family=Lato:wght@400;700&family=Source+Code+Pro&display=swap")
      #html.link(rel: "stylesheet", href: "/styles/global.css")
      #html.title[#title]
    ]
    #html.body[
      #html.div(class: "glow")
      #header()
      #body
      #footer()
    ]
  ]
]
