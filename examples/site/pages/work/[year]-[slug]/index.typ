#import "/lib.typ": get-collection-ids, get-entry, route-params
#import "/templates/site.typ": site

#metadata(
  get-collection-ids("projects").map(id => {
    let parts = id.split("/")
    (year: parts.first(), slug: parts.last())
  })
) <aster-route>

#let year = route-params.at("year", default: "")
#let slug = route-params.at("slug", default: "")
#let entry = get-entry("projects", year + "/" + slug)

#if entry != none [
  #let meta = entry.metadata()

  #show: site.with(
    title: meta.title,
    description: meta.summary,
  )
  #show heading.where(level: 1): it => html.elem("header")[
    #html.elem("h1")[#it.body]
    #html.elem("p")[#meta.summary]
  ]

  #html.elem("article")[#entry.render()]
  #html.elem("nav", attrs: ("aria-label": "Project navigation"))[
    #link("/")[Return to the project list]
  ]
]
