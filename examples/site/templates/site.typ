#import "/components/navigation.typ": navigation
#import "/lib.typ": aster-version, settings

#let site(
  title: settings.site.title,
  stylesheet: "/styles/site.css",
  description: settings.site.description,
  body,
) = {
  let generator = if aster-version == none { "Aster" } else { "Aster " + aster-version }
  let document-title = if title == settings.site.title {
    title
  } else {
    title + " | " + settings.site.title
  }
  let document = {
    show heading.where(level: 1): it => html.elem("h1")[#it.body]
    show heading.where(level: 2): it => html.elem("h2")[#it.body]
    show heading.where(level: 3): it => html.elem("h3")[#it.body]
    body
  }

  html.html({
    html.head[
      #html.meta(charset: "utf-8")
      #html.meta(
        name: "viewport",
        content: "width=device-width, initial-scale=1",
      )
      #html.meta(name: "generator", content: generator)
      #html.meta(name: "description", content: description)
      #html.title(document-title)
      #html.elem("link", attrs: (
        rel: "alternate",
        type: "application/atom+xml",
        title: settings.site.title,
        href: settings.site.url + "atom.xml",
      ))
      #html.link(rel: "stylesheet", href: stylesheet)
    ]
    html.body[
      #navigation()
      #html.elem("main")[#document]
      #html.elem("footer")[
        #html.elem("p")[This example was written by #settings.author.name.]
      ]
    ]
  })
}
