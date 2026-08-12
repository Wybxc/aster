#import "/lib.typ": adjacent-posts, canonical-url, date-label, date-value, post-url, settings
#import "/components/icons.typ": arrow-left-icon, arrow-right-icon, arrow-up-icon
#import "/components/prose.typ": prose
#import "/components/tags.typ": tag-list
#import "site.typ": site

#let article(body, item: none) = {
  let post = item.metadata
  let adjacent = adjacent-posts(item.entry.id)
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
  let head = [
    #html.script(json.encode(structured), type: "application/ld+json")
    #html.link(rel: "stylesheet", href: "./article.css")
    #html.script(src: "./article.js", defer: true)
  ]

  show: site.with(
    title: post.title,
    description: post.description,
    author: post.author,
    canonical: post.canonical,
    kind: "article",
    extra-head: head,
  )

  html.progress(
    id: "reading-progress",
    max: 100,
    value: 0,
    aria-label: "Reading progress",
  )[]
  html.main(id: "main-content")[
    #html.a(href: "/posts/")[#arrow-left-icon Back to posts]
    #html.header[
      #html.h1[#post.title]
      #html.p[
        #html.time(datetime: date-value(post.date))[#date-label(post.date)]
        #if post.modified != none [
          #html.span[Updated #date-label(post.modified)]
        ]
      ]
    ]
    #html.article(id: "article")[#prose(body)] <aster-content>
    #html.button(
      id: "back-to-top",
      type: "button",
      title: "Back to top",
      aria-label: "Back to top",
      hidden: true,
    )[#arrow-up-icon]
    #tag-list(post.tags, extra-class: "article-tags")
    #html.nav(aria-label: "Adjacent posts")[
      #if adjacent.older != none {
        html.a(
          href: post-url(adjacent.older.entry.id),
          rel: "prev",
        )[
          #arrow-left-icon
          #html.span[#html.small[Older post]#adjacent.older.metadata.title]
        ]
      }
      #if adjacent.newer != none {
        html.a(
          href: post-url(adjacent.newer.entry.id),
          rel: "next",
        )[
          #html.span[#html.small[Newer post]#adjacent.newer.metadata.title]
          #arrow-right-icon
        ]
      }
    ]
  ]
}
