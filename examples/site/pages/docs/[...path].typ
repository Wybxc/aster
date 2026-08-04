#import "/lib.typ": get-collection-ids, get-entry, root-prefix
#import "/templates/site.typ": site

#metadata(
  get-collection-ids("docs").map(path => (path: path))
) <route>

#let path = sys.inputs.at("path", default: "")
#let entry = get-entry("docs", path)

#if entry != none [
  #let meta = entry.metadata()
  #let root = root-prefix(path.split("/").len())

  #show: site.with(
    title: meta.title,
    root: root,
    description: meta.summary,
  )
  #show heading.where(level: 1): it => html.elem("header")[
    #html.elem("h1")[#it.body]
    #html.elem("p")[#meta.summary]
  ]

  #html.elem("section", attrs: ("aria-label": "Documentation"))[
    #html.elem("aside")[
      #html.elem("nav", attrs: ("aria-label": "Documentation navigation"))[
        #link(root + "docs/guides/getting-started.html")[Getting started]
        #link(root + "docs/guides/content-collections.html")[Collections]
        #link(root + "docs/reference/routing.html")[Routing]
        #link(root + "docs/reference/configuration.html")[Configuration]
      ]
    ]
    #html.elem("article")[#entry.render()]
  ]
]
