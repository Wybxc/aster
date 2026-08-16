#import "/lib.typ": aster-version, canonical-url, route-section, settings
#import "/components/footer.typ": footer
#import "/components/header.typ": header

#let site(
  title: settings.site.title,
  description: settings.site.description,
  author: settings.site.author,
  canonical: none,
  kind: "website",
  extra-head: none,
  body,
) = {
  let document-title = if title == settings.site.title {
    title
  } else {
    title + " | " + settings.site.title
  }
  let canonical = if canonical == none { canonical-url() } else { canonical }
  let generator = if aster-version == none { "Aster" } else { "Aster " + aster-version }

  html.html(lang: settings.site.language)[
    #html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.meta(name: "generator", content: generator)
      #html.meta(name: "description", content: description)
      #html.meta(name: "author", content: author)
      #html.meta(name: "theme-color", content: "")
      #html.elem("meta", attrs: (property: "og:type", content: kind))
      #html.elem("meta", attrs: (property: "og:site_name", content: settings.site.title))
      #html.elem("meta", attrs: (property: "og:title", content: document-title))
      #html.elem("meta", attrs: (property: "og:description", content: description))
      #html.elem("meta", attrs: (property: "og:url", content: canonical))
      #html.link(rel: "canonical", href: canonical)
      #html.link(rel: "icon", type: "image/svg+xml", href: "/assets/favicon.svg")
      #html.link(
        rel: "alternate",
        type: "application/atom+xml",
        title: settings.site.title,
        href: settings.site.url + "atom.xml",
      )
      #html.title(document-title)
      #html.script(src: "./theme-init.js")
      #html.link(rel: "stylesheet", href: "./site.css")
      #if extra-head != none { extra-head }
    ]
    #html.body[
      #header(active: route-section())
      #body
      #footer()
    ]
  ]
}
