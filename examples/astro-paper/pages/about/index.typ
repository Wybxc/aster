#import "/lib.typ": get-entry
#import "/templates/site.typ": site

#let entry = get-entry("pages", "about")
#let metadata = if entry == none {
  (title: "About", description: "About AsterPaper")
} else {
  entry.metadata()
}

#show: site.with(
  title: metadata.title,
  description: metadata.description,
  path: "/about/",
  active: "about",
)

#html.elem("main", attrs: (id: "main-content", class: "app-main standard-page"))[
  #html.elem("h1")[#metadata.title]
  #html.elem("p", attrs: (class: "page-description"))[#metadata.description]
  #if entry != none {
    html.elem("article", attrs: (class: "article-prose"))[#entry.render()]
  }
]
