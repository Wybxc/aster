#import "/lib.typ": get-entry, projects, route-param
#import "/templates/site.typ": site

#metadata(projects().map(item => (slug: item.entry.id))) <aster-route>

#let entry = get-entry("work", route-param("slug", default: ""))
#if entry != none [
  #let data = entry.metadata()
  #show: site.with(title: data.title + " | Mira Chen", description: data.description, active: "work")
  #html.main(class: "wrapper project-page")[
    #html.header(class: "project-header")[
      #html.a(href: "/work/")[#("<- Back to work")]
      #html.h1[#data.title]
      #html.div(class: "details")[
        #html.p[#data.description]
        #html.div(class: "tags")[
          #for tag in data.tags { html.span(class: "pill")[#tag] }
        ]
      ]
    ]
    #html.article(class: "project-content")[
      #html.img(src: data.image, alt: data.image_alt)
      #entry.render()
    ]
  ]
]
