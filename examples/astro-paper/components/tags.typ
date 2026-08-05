#import "/lib.typ": tag-slug

#let tag-list(items, counts: false) = [
  #metadata("./tags.css") <aster-style>
  #html.elem("nav", attrs: (
    "aria-label": if counts { "All tags" } else { "Tags" },
  ))[
    #html.elem("ul")[
      #for item in items {
        let name = if type(item) == str { item } else { item.name }
        let slug = if type(item) == str { tag-slug(item) } else { item.slug }
        html.elem("li")[
          #html.elem("a", attrs: (href: "/tags/" + slug + "/"))[
            #name
            #if counts { html.elem("span")[(#item.count)] }
          ]
        ]
      }
    ]
  ]
]
