#import "/lib.typ": get-entry, published-posts, route-param
#import "/templates/article.typ": article

#metadata(
  published-posts().map(item => (slug: item.entry.id))
) <aster-route>

#let slug = route-param("slug", default: "")
#let entry = get-entry("posts", slug)

#if entry != none [
  #let item = (entry: entry, metadata: entry.metadata())
  #show: article.with(item: item)
  #entry.render()
]
