#import "/lib.typ": settings
#import "/components/header.typ": header
#import "/components/footer.typ": footer

#let site(body, title: settings.site.title, description: settings.site.description, active: "") = [
  #html.html(lang: settings.site.language)[
    #html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.meta(name: "description", content: description)
      #html.elem("link", attrs: (rel: "icon", href: "/assets/favicon.svg"))
      #html.elem("link", attrs: (rel: "alternate", type: "application/rss+xml", href: "/rss.xml", title: settings.site.title))
      #html.link(rel: "stylesheet", href: "/styles/global.css")
      #html.title[#title]
    ]
    #html.body[
      #header(active: active)
      #body
      #footer()
    ]
  ]
]
