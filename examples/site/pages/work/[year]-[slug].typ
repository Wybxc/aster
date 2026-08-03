#import "/lib/aster/content.typ": get-collection-ids, get-entry
#import "/site.typ": site

#metadata(
  get-collection-ids("projects").map(id => {
    let parts = id.split("/")
    (year: parts.first(), slug: parts.last())
  })
) <route>

#let year = sys.inputs.at("year", default: "")
#let slug = sys.inputs.at("slug", default: "")
#let entry = get-entry("projects", year + "/" + slug)

#if entry != none [
  #let meta = entry.metadata()

  #show: site.with(
    title: meta.title,
    root: "../",
    description: meta.summary,
  )
  #show heading.where(level: 1): it => html.elem("header")[
    #html.elem("h1")[#it.body]
    #html.elem("p")[#meta.summary]
  ]

  #html.elem("article")[#entry.render()]
  #html.elem("nav", attrs: ("aria-label": "Project navigation"))[
    #link("../index.html")[Return to the project list]
  ]
]
