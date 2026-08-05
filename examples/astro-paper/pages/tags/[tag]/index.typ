#import "/lib.typ": all-tags, page-count, page-items, posts-with-tag, route-params
#import "/components/pagination.typ": pagination
#import "/components/page-header.typ": page-header
#import "/components/post-list.typ": post-list
#import "/templates/site.typ": site

#metadata(all-tags().map(tag => (tag: tag.slug))) <aster-route>

#let tag = route-params.at("tag", default: "")
#let info = all-tags().find(item => item.slug == tag)
#let name = if info == none { tag } else { info.name }
#let posts = posts-with-tag(tag)
#let total = page-count(posts)

#show: site.with(
  title: "Tag: " + name,
  description: "Posts tagged " + name + ".",
)

#html.main(id: "main-content")[
  #page-header([Tag: #name], [All posts filed under #name.])
  #post-list(page-items(posts, 1))
  #pagination(1, total, "/tags/" + tag + "/")
]
