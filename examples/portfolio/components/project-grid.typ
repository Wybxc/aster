#import "/lib.typ": project-url

#let project-grid(items) = html.ul(class: "project-grid")[
  #for item in items {
    let data = item.metadata
    html.li[
      #html.a(class: "project-card", href: project-url(item.entry.id))[
        #html.img(src: data.image, alt: data.image_alt, loading: "lazy")
        #html.span[#data.title]
      ]
    ]
  }
]
