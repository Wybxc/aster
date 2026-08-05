#import "/lib.typ": get-collection-ids, get-entry, root-prefix
#import "/templates/site.typ": site

#metadata(
  get-collection-ids("docs").map(path => (path: path))
) <aster-route>

#let path = sys.inputs.at("path", default: "")
#let entry = get-entry("docs", path)

#if entry != none [
  #let meta = entry.metadata()
  #let root = root-prefix(path.split("/").len() + 1)

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
        #link(root + "docs/guides/getting-started/")[Getting started]
        #link(root + "docs/guides/content-collections/")[Collections]
        #link(root + "docs/reference/routing/")[Routing]
        #link(root + "docs/reference/configuration/")[Configuration]
      ]
    ]
    #html.elem("article")[#entry.render()]
  ]
]
