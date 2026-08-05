#import "/lib.typ": adjacent-posts, canonical-url, date-label, post-url, settings
#import "/components/icons.typ": arrow-left-icon, arrow-right-icon, arrow-up-icon
#import "/components/prose.typ": prose
#import "/components/tags.typ": tag-list
#import "site.typ": site

#let article(body, item: none) = {
  let post = item.metadata
  let adjacent = adjacent-posts(item.entry.id)
  let path = post-url(item.entry.id)
  let author = ("@type": "Person", name: post.author)
  let profile = settings.site.at("profile", default: none)
  if post.author == settings.site.author and profile != none {
    author.insert("url", profile)
  }
  let structured = (
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: post.title,
    description: post.description,
    datePublished: post.date,
    author: (author,),
  )
  let head = html.script(json.encode(structured), type: "application/ld+json")

  show: site.with(
    title: post.title,
    description: post.description,
    author: post.author,
    path: path,
    canonical: post.canonical,
    active: "posts",
    kind: "article",
    extra-head: head,
  )

  [#metadata("./article.css") <aster-style>]
  [#metadata("./article.js") <aster-module>]
  html.elem("progress", attrs: (
    id: "reading-progress",
    max: "100",
    value: "0",
    "aria-label": "Reading progress",
  ))[]
  html.elem("main", attrs: (id: "main-content"))[
    #html.elem("a", attrs: (href: "/posts/"))[#arrow-left-icon Back to posts]
    #html.elem("header")[
      #html.elem("h1")[#post.title]
      #html.elem("p")[
        #html.elem("time", attrs: (datetime: post.date))[#date-label(post.date)]
        #if post.modified != none [
          #html.elem("span")[Updated #date-label(post.modified)]
        ]
      ]
    ]
    #prose(body, id: "article")
    #html.elem("button", attrs: (
      id: "back-to-top",
      type: "button",
      title: "Back to top",
      "aria-label": "Back to top",
      hidden: "",
    ))[#arrow-up-icon]
    #tag-list(post.tags)
    #html.elem("nav", attrs: ("aria-label": "Adjacent posts"))[
      #if adjacent.older != none {
        html.elem("a", attrs: (
          href: post-url(adjacent.older.entry.id),
          rel: "prev",
        ))[
          #arrow-left-icon
          #html.elem("span")[#html.elem("small")[Older post]#adjacent.older.metadata.title]
        ]
      }
      #if adjacent.newer != none {
        html.elem("a", attrs: (
          href: post-url(adjacent.newer.entry.id),
          rel: "next",
        ))[
          #html.elem("span")[#html.elem("small")[Newer post]#adjacent.newer.metadata.title]
          #arrow-right-icon
        ]
      }
    ]
  ]
}
