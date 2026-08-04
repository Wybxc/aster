#import "/lib.typ": all-tags
#import "/templates/site.typ": site

#let tags = all-tags()

#show: site.with(
  title: "Tags",
  description: "Browse AsterPaper articles by topic.",
  path: "/tags/",
  active: "tags",
)

#html.elem("main", attrs: (id: "main-content", class: "app-main standard-page"))[
  #html.elem("h1")[Tags]
  #html.elem("p", attrs: (class: "page-description"))[All topics used across published articles.]
  #html.elem("ul", attrs: (class: "tag-cloud"))[
    #for tag in tags {
      html.elem("li")[
        #html.elem("a", attrs: (class: "tag-link large", href: "/tags/" + tag.slug + "/"))[
          #tag.name #html.elem("span", attrs: (class: "tag-count"))[(#tag.count)]
        ]
      ]
    }
  ]
]
