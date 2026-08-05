#import "/lib.typ": canonical-url, settings
#import "/components/footer.typ": footer
#import "/components/header.typ": header

#let site(
  title: settings.site.title,
  description: settings.site.description,
  author: settings.site.author,
  path: "/",
  canonical: none,
  active: "",
  kind: "website",
  extra-head: none,
  body,
) = {
  let document-title = if title == settings.site.title {
    title
  } else {
    title + " | " + settings.site.title
  }
  let canonical = if canonical == none { canonical-url(path) } else { canonical }

  html.html(lang: settings.site.language)[
    #html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.meta(name: "generator", content: "Aster 0.1.0")
      #html.meta(name: "description", content: description)
      #html.meta(name: "author", content: author)
      #html.meta(name: "theme-color", content: "")
      #html.elem("meta", attrs: (property: "og:type", content: kind))
      #html.elem("meta", attrs: (property: "og:site_name", content: settings.site.title))
      #html.elem("meta", attrs: (property: "og:title", content: document-title))
      #html.elem("meta", attrs: (property: "og:description", content: description))
      #html.elem("meta", attrs: (property: "og:url", content: canonical))
      #html.elem("link", attrs: (rel: "canonical", href: canonical))
      #html.elem("link", attrs: (rel: "icon", type: "image/svg+xml", href: "/assets/favicon.svg"))
      #html.elem("link", attrs: (
        rel: "alternate",
        type: "application/rss+xml",
        title: settings.site.title,
        href: settings.site.url + "rss.xml",
      ))
      #html.title(document-title)
      #if extra-head != none { extra-head }
      #html.script(read("./theme-init.js"))
      #html.elem("link", attrs: (rel: "tailwind", href: "/styles/site.css"))
    ]
    #html.body[
      #header(active: active)
      #body
      #footer()
    ]
  ]
}
