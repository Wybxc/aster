#let settings = sys.inputs

#let root-prefix(depth) = range(depth).map(_ => "../").join()

#let site(
  title: settings.site.title,
  root: "",
  stylesheet: "/styles/site.css",
  description: settings.site.description,
  body,
) = {
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
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.meta(name: "description", content: description)
      #html.title(document-title)
      #html.elem("link", attrs: (
        rel: "alternate",
        type: "application/rss+xml",
        title: settings.site.title,
        href: settings.site.url + "rss.xml",
      ))
      #html.elem("link", attrs: (rel: "css", href: stylesheet))
    ]
    html.body[
      #html.elem("header")[
        #html.elem("nav", attrs: ("aria-label": "Primary navigation"))[
          #link(root + "index.html")[*#settings.site.title*]
          #for item in settings.navigation {
            link(root + item.href)[#item.label]
          }
        ]
      ]
      #html.elem("main")[#document]
      #html.elem("footer")[
        #html.elem("p")[This example was written by #settings.author.name.]
      ]
    ]
  })
}
