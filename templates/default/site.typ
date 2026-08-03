#let settings = sys.inputs

#let site(
  title: settings.site.title,
  description: settings.site.description,
  body,
) = html.html({
  html.head[
    #html.meta(charset: "utf-8")
    #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
    #html.meta(name: "description", content: description)
    #html.title(title)
    #html.elem("link", attrs: (rel: "css", href: "/styles/site.css"))
  ]
  html.body[
    #html.elem("header")[
      #html.elem("nav")[#link("/")[*#settings.site.title*]]
    ]
    #html.elem("main")[#body]
    #html.elem("footer")[Built with Aster and Typst.]
  ]
})
