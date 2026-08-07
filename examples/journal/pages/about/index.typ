#import "/lib.typ": get-entry
#import "/components/page-header.typ": page-header
#import "/components/prose.typ": prose
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
)

#html.main(id: "main-content")[
  #page-header(metadata.title, metadata.description)
  #if entry != none {
    html.article[#prose(entry.render())]
  }
]
