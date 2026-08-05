#import "/components/navigation.typ": navigation
#import "/lib.typ": aster-version, settings

#let site(
  title: settings.site.title,
  description: settings.site.description,
  body,
) = {
  let generator = if aster-version == none { "Aster" } else { "Aster " + aster-version }
  html.html({
    html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.meta(name: "generator", content: generator)
      #html.meta(name: "description", content: description)
      #html.title(title)
      #html.link(rel: "stylesheet", href: "/styles/site.css")
    ]
    html.body[
      #navigation()
      #html.elem("main")[#body]
      #html.elem("footer")[Built with Aster and Typst.]
    ]
  })
}
