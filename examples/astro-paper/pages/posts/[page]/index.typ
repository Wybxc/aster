#import "/lib.typ": page-count, page-items, published-posts
#import "/components/pagination.typ": pagination
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
) <route>

#let current = int(sys.inputs.at("page", default: "1"))

#show: site.with(
  title: "Posts - Page " + str(current),
  description: "All articles published on AsterPaper.",
  path: "/posts/" + str(current) + "/",
  active: "posts",
)

#html.elem("main", attrs: (id: "main-content", class: "app-main listing-page"))[
  #html.elem("h1")[Posts]
  #html.elem("p", attrs: (class: "page-description"))[Page #current of #total.]
  #post-list(page-items(posts, current))
  #pagination(current, total, "/posts/")
]
