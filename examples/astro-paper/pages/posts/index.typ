#import "/lib.typ": page-count, page-items, published-posts
#import "/components/pagination.typ": pagination
#import "/components/post-list.typ": post-list
#import "/templates/site.typ": site

#let posts = published-posts()
#let total = page-count(posts)

#show: site.with(
  title: "Posts",
  description: "All articles published on AsterPaper.",
  path: "/posts/",
  active: "posts",
)

#html.elem("main", attrs: (id: "main-content", class: "app-main listing-page"))[
  #html.elem("h1")[Posts]
  #html.elem("p", attrs: (class: "page-description"))[All articles in reverse chronological order.]
  #post-list(page-items(posts, 1))
  #pagination(1, total, "/posts/")
]
