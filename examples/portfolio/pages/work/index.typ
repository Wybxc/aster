#import "/lib.typ": projects
#import "/components/project-grid.typ": project-grid
#import "/templates/site.typ": site

#show: site.with(title: "My Work | Jeanine White", description: "Jeanine White's recent projects.", active: "work")

#html.main(class: "wrapper work-page")[
  #html.header(class: "page-hero")[
    #html.h1[My Work]
    #html.p(class: "tagline")[See my most recent projects below to get an idea of my past experience.]
  ]
  #project-grid(projects())
  #html.aside(class: "contact")[
    #html.h2[Interested in working together?]
    #html.a(class: "button-link", href: "mailto:me@example.com")[Send me a message #("->")]
  ]
]
