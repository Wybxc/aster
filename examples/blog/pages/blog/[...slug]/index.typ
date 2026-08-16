#import "/lib.typ": get-entry, posts, route-param
#import "/templates/post.typ": post

#metadata(posts().map(item => (slug: item.entry.id))) <aster-route>

#let entry = get-entry("posts", route-param("slug", default: ""))
#if entry != none [
  #let item = (entry: entry, metadata: entry.metadata())
  #show: post.with(item: item)
  #entry.render()
]
