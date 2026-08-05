#import "/lib.typ": date-label, post-url
#import "tags.typ": tag-list

#let post-card(item, heading-level: 2) = {
  let post = item.metadata
  let heading-tag = "h" + str(heading-level)
  html.elem("li")[
    #html.elem("article")[
      #html.elem("header")[
        #html.elem(heading-tag)[
          #html.elem("a", attrs: (href: post-url(item.entry.id)))[#post.title]
        ]
        #html.elem("p")[
          #html.elem("time", attrs: (datetime: post.date))[#date-label(post.date)]
          #if post.modified != none [#html.elem("span")[Updated]]
        ]
      ]
      #html.elem("p")[#post.description]
      #tag-list(post.tags)
    ]
  ]
}

#let post-list(items, heading-level: 2) = [
  #metadata("./post-list.css") <aster-style>
  #html.elem("ol", attrs: ("aria-label": "Posts"))[
    #for item in items {
      post-card(item, heading-level: heading-level)
    }
  ]
]
