#import "/lib.typ": page-count, page-items, published-posts, route-param
#import "/components/pagination.typ": pagination
#import "/components/page-header.typ": page-header
#import "/components/post-list.typ": post-list
#import "/templates/site.typ": site

#let posts = published-posts()
#let total = page-count(posts)
#metadata(
  if total > 1 {
    range(2, total + 1).map(page => (page: str(page)))
  } else {
    ()
  }
) <aster-route>

#let current = int(route-param("page", default: "1"))

#show: site.with(
  title: "Posts - Page " + str(current),
  description: "All articles published on AsterPaper.",
)

#html.main(id: "main-content")[
  #page-header([Posts], [Page #current of #total.])
  #post-list(page-items(posts, current))
  #pagination(current, total, "/posts/")
]
