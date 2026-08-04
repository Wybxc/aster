#import "/lib.typ": adjacent-posts, canonical-url, date-label, post-url, settings, tag-slug
#import "/components/icons.typ": arrow-left-icon, arrow-right-icon, arrow-up-icon
#import "site.typ": site

#let article(body, item: none) = {
  let metadata = item.metadata
  let adjacent = adjacent-posts(item.entry.id)
  let path = post-url(item.entry.id)
  let author = ("@type": "Person", name: metadata.author)
  let profile = settings.site.at("profile", default: none)
  if metadata.author == settings.site.author and profile != none {
    author.insert("url", profile)
  }
  let structured = (
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: metadata.title,
    description: metadata.description,
    datePublished: metadata.date,
    author: (author,),
  )
  let head = html.script(json.encode(structured), type: "application/ld+json")

  show: site.with(
    title: metadata.title,
    description: metadata.description,
    author: metadata.author,
    path: path,
    canonical: metadata.canonical,
    active: "posts",
    kind: "article",
    extra-head: head,
  )

  html.elem("div", attrs: (id: "reading-progress", class: "reading-progress"))[]
  html.elem("main", attrs: (id: "main-content", class: "app-main article-main"))[
    #html.elem("a", attrs: (class: "back-link", href: "/posts/"))[#arrow-left-icon Back to posts]
    #html.elem("header", attrs: (class: "article-header"))[
      #html.elem("h1")[#metadata.title]
      #html.elem("div", attrs: (class: "post-meta"))[
        #html.elem("time", attrs: (datetime: metadata.date))[#date-label(metadata.date)]
        #if metadata.modified != none [
          #html.elem("span")[Updated #date-label(metadata.modified)]
        ]
      ]
    ]
    #html.elem("article", attrs: (id: "article", class: "article-prose"))[
      #body
    ]
    #html.elem("button", attrs: (
      id: "back-to-top",
      class: "back-to-top icon-button",
      type: "button",
      title: "Back to top",
      "aria-label": "Back to top",
    ))[#arrow-up-icon]
    #html.elem("ul", attrs: (class: "article-tags", "aria-label": "Tags"))[
      #for tag in metadata.tags {
        html.elem("li")[
          #html.elem("a", attrs: (
            class: "tag-link",
            href: "/tags/" + tag-slug(tag) + "/",
          ))[#tag]
        ]
      }
    ]
    #html.elem("nav", attrs: (class: "adjacent-posts", "aria-label": "Adjacent posts"))[
      #if adjacent.older != none {
        html.elem("a", attrs: (href: post-url(adjacent.older.entry.id), class: "adjacent-link older"))[
          #arrow-left-icon
          #html.elem("span")[#html.elem("small")[Older post]#adjacent.older.metadata.title]
        ]
      } else { html.elem("span")[] }
      #if adjacent.newer != none {
        html.elem("a", attrs: (href: post-url(adjacent.newer.entry.id), class: "adjacent-link newer"))[
          #html.elem("span")[#html.elem("small")[Newer post]#adjacent.newer.metadata.title]
          #arrow-right-icon
        ]
      }
    ]
  ]
}
