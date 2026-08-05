#import "/lib.typ": tag-slug

#let tag-list(items, counts: false, extra-class: none) = {
  let entries = items.map(item => {
    let name = if type(item) == str { item } else { item.name }
    let slug = if type(item) == str { tag-slug(item) } else { item.slug }
    list.item(
      html.a(href: "/tags/" + slug + "/")[
        #name
        #if counts { html.span[(#item.count)] }
      ],
    )
  })
  [
    #metadata("./tags.css") <aster-style>
    #html.nav(
      class: (
        "tag-list"
        + if counts { " tag-cloud" } else { "" }
        + if extra-class == none { "" } else { " " + extra-class }
      ),
      aria-label: if counts { "All tags" } else { "Tags" },
    )[
      #list(..entries)
    ]
  ]
}
