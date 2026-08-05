#import "/lib.typ": date-label, date-value, post-url
#import "tags.typ": tag-list

#let post-card(item, heading-level: 2) = {
  assert(heading-level >= 2 and heading-level <= 6, message: "post heading level must be between 2 and 6")
  let post = item.metadata
  let title = link(post-url(item.entry.id))[#post.title]
  html.li[
    #html.article[
      #html.header[
        #heading(level: heading-level - 1)[#title]
        #html.p[
          #html.time(datetime: date-value(post.date))[#date-label(post.date)]
          #if post.modified != none [#html.span[Updated]]
        ]
      ]
      #html.p[#post.description]
      #tag-list(post.tags)
    ]
  ]
}

#let post-list(items, heading-level: 2) = [
  #metadata("./post-list.css") <aster-style>
  #html.ol(class: "post-list", aria-label: "Posts")[
    #for item in items {
      post-card(item, heading-level: heading-level)
    }
  ]
]
