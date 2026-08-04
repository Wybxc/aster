#import "/lib.typ": get-entry, published-posts
#import "/templates/article.typ": article

#metadata(
  published-posts().map(item => (slug: item.entry.id))
) <route>

#let slug = sys.inputs.at("slug", default: "")
#let entry = get-entry("posts", slug)

#if entry != none [
  #let item = (entry: entry, metadata: entry.metadata())
  #show: article.with(item: item)
  #entry.render()
]
