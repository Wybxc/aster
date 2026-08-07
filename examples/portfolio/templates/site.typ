#import "/lib.typ": settings
#import "/components/footer.typ": footer
#import "/components/nav.typ": nav

#let site(body, title: settings.site.title + ": Personal Site", description: settings.site.description, active: "") = [
  #html.html(lang: settings.site.language)[
    #html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.meta(name: "description", content: description)
      #html.elem("link", attrs: (rel: "icon", href: "/assets/favicon.svg"))
      #html.elem("link", attrs: (rel: "preconnect", href: "https://fonts.googleapis.com"))
      #html.link(rel: "stylesheet", href: "https://fonts.googleapis.com/css2?family=Public+Sans:wght@400;700&family=Rubik:wght@500;600&display=swap")
      #html.link(rel: "stylesheet", href: "/styles/global.css")
      #html.title[#title]
    ]
    #html.body[
      #nav(active: active)
      #body
      #footer()
    ]
  ]
]
