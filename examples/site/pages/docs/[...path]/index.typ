#import "/lib.typ": get-collection-ids, get-entry, route-params
#import "/templates/site.typ": site

#metadata(
  get-collection-ids("docs").map(path => (path: path))
) <aster-route>

#let path = route-params.at("path", default: "")
#let entry = get-entry("docs", path)

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

  #html.elem("section", attrs: ("aria-label": "Documentation"))[
    #html.elem("aside")[
      #html.elem("nav", attrs: ("aria-label": "Documentation navigation"))[
        #link("/docs/guides/getting-started/")[Getting started]
        #link("/docs/guides/content-collections/")[Collections]
        #link("/docs/reference/routing/")[Routing]
        #link("/docs/reference/configuration/")[Configuration]
      ]
    ]
    #html.elem("article")[#entry.render()]
  ]
]
