#import "/lib.typ": all-tags, page-count, page-items, posts-with-tag
#import "/components/pagination.typ": pagination
#import "/components/page-header.typ": page-header
#import "/components/post-list.typ": post-list
#import "/templates/site.typ": site

#let routes = {
  let values = ()
  for tag in all-tags() {
    let total = page-count(posts-with-tag(tag.slug))
    if total > 1 {
      values += range(2, total + 1).map(page => (tag: tag.slug, page: str(page)))
    }
  }
  values
}
#metadata(routes) <aster-route>

#let tag = sys.inputs.at("tag", default: "")
#let current = int(sys.inputs.at("page", default: "1"))
#let info = all-tags().find(item => item.slug == tag)
#let name = if info == none { tag } else { info.name }
#let posts = posts-with-tag(tag)
#let total = page-count(posts)

#show: site.with(
  title: "Tag: " + name + " - Page " + str(current),
  description: "Posts tagged " + name + ".",
  path: "/tags/" + tag + "/" + str(current) + "/",
  active: "tags",
)

#html.elem("main", attrs: (id: "main-content"))[
  #page-header([Tag: #name], [Page #current of #total.])
  #post-list(page-items(posts, current))
  #pagination(current, total, "/tags/" + tag + "/")
]
