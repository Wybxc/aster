#import "/lib/aster/content.typ": get-collection-ids, get-entry
#import "/site.typ": site

#metadata(
  get-collection-ids("journal").map(slug => (slug: slug))
) <route>

#let slug = sys.inputs.at("slug", default: "")
#let entry = get-entry("journal", slug)

#if entry != none [
  #let rendered = entry.render()
  #let meta = rendered.metadata

  #show: site.with(
    title: meta.title,
    root: "../",
    stylesheet: "../style.css",
    description: meta.summary,
  )
  #show heading.where(level: 1): it => html.elem("header")[
    #html.elem("h1")[#it.body]
    #html.elem("p")[#meta.summary]
  ]

  #html.elem("article")[#rendered.content]
  #html.elem("nav", attrs: ("aria-label": "Journal navigation"))[
    #link("../index.html")[Return to all journal entries]
  ]
]
