#import "/lib.typ": date-label, post-url, tag-slug

#let post-card(item, heading-level: 2) = {
  let metadata = item.metadata
  let heading-tag = "h" + str(heading-level)
  html.elem("li", attrs: (class: "post-card"))[
    #html.elem("a", attrs: (class: "post-card-link", href: post-url(item.entry.id)))[
      #html.elem(heading-tag)[#metadata.title]
    ]
    #html.elem("div", attrs: (class: "post-meta"))[
      #html.elem("time", attrs: (datetime: metadata.date))[#date-label(metadata.date)]
      #if metadata.modified != none [
        #html.elem("span", attrs: (class: "updated"))[Updated]
      ]
    ]
    #html.elem("p")[#metadata.description]
    #html.elem("ul", attrs: (class: "tag-row", "aria-label": "Tags"))[
      #for tag in metadata.tags {
        html.elem("li")[
          #html.elem("a", attrs: (
            class: "tag-link",
            href: "/tags/" + tag-slug(tag) + "/",
          ))[#tag]
        ]
      }
    ]
  ]
}

#let post-list(items, heading-level: 2) = html.elem("ul", attrs: (class: "post-list"))[
  #for item in items {
    post-card(item, heading-level: heading-level)
  }
]
