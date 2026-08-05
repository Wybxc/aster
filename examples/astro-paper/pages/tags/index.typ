#import "/lib.typ": all-tags
#import "/components/page-header.typ": page-header
#import "/components/tags.typ": tag-list
#import "/templates/site.typ": site

#let tags = all-tags()

#show: site.with(
  title: "Tags",
  description: "Browse AsterPaper articles by topic.",
  path: "/tags/",
  active: "tags",
)

#html.elem("main", attrs: (id: "main-content"))[
  #page-header([Tags], [All topics used across published articles.])
  #tag-list(tags, counts: true)
]
