#import "/lib/aster/content.typ": get-collection
#import "/site.typ": site

#let journal = {
  get-collection("journal")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .sorted(key: item => item.metadata.date)
    .rev()
}
#let projects = {
  get-collection("projects")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .sorted(key: item => item.entry.id)
    .rev()
}
#let docs = {
  get-collection("docs")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .sorted(key: item => item.metadata.order)
}

#show: site.with(description: sys.inputs.site.description)

#html.elem("header")[
  #heading(level: 1)[Aster Field Notes]

  Aster Field Notes is a complete reference site that shows how Typst
  templates, lazy content collections, dynamic routes, and tracked builds work
  together in one Aster project.

  #html.elem("nav", attrs: ("aria-label": "Introduction"))[
    #link("docs/guides/getting-started.html")[Read the complete guide]
    #link("features.html")[Review the Typst examples]
  ]
]

#figure(
  image(
    "/assets/pipeline.svg",
    width: 100%,
    alt: "The diagram shows how Aster turns Typst sources into a deterministic static output tree.",
  ),
  caption: [
    The diagram shows how Aster validates routes, compiles pages through a
    tracked Typst world, and publishes a deterministic static tree.
  ],
  supplement: none,
  numbering: none,
)

#html.elem("section")[
  #heading(level: 2)[The journal explains how Aster performs incremental builds.]
  Each listing calls the entry's lazy `metadata` function without retaining
  article bodies.
  #list(
    ..journal.map(item => {
      let meta = item.metadata
      html.elem("article")[
        #heading(level: 3)[
          #link("journal/" + item.entry.id + ".html")[#meta.title]
        ]
        #html.elem("p")[#meta.summary]
      ]
    }),
  )
]

#html.elem("section")[
  #heading(level: 2)[Project pages combine the year and slug in one route.]
  Each nested project id is split into the parameters required by
  `work/[year]-[slug].typ` before Aster renders the page.
  #list(
    ..projects.map(item => {
      let parts = item.entry.id.split("/")
      let meta = item.metadata
      html.elem("article")[
        #heading(level: 3)[
          #link("work/" + parts.first() + "-" + parts.last() + ".html")[#meta.title]
        ]
        #html.elem("p")[#meta.summary]
      ]
    }),
  )
]

#html.elem("section")[
  #heading(level: 2)[Documentation entries preserve their nested collection paths.]
  The spread route passes each complete documentation id to one template, so
  the generated file keeps the same nested path.
  #list(
    ..docs.map(item => {
      let meta = item.metadata
      html.elem("article")[
        #heading(level: 3)[
          #link("docs/" + item.entry.id + ".html")[#meta.title]
        ]
        #html.elem("p")[#meta.summary]
      ]
    }),
  )
]
