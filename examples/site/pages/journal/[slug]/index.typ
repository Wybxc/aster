#import "/lib.typ": get-collection-ids, get-entry, root-prefix, route-params
#import "/templates/site.typ": site

#metadata(
  get-collection-ids("journal").map(slug => (slug: slug))
) <aster-route>

#let slug = route-params.at("slug", default: "")
#let entry = get-entry("journal", slug)

#if entry != none [
  #let meta = entry.metadata()
  #let root = root-prefix()

  #show: site.with(
    title: meta.title,
    description: meta.summary,
  )
  #show heading.where(level: 1): it => html.elem("header")[
    #html.elem("h1")[#it.body]
    #html.elem("p")[#meta.summary]
  ]

  #html.elem("article")[#entry.render()]
  #html.elem("nav", attrs: ("aria-label": "Journal navigation"))[
    #link(root)[Return to all journal entries]
  ]
]
