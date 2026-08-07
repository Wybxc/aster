#import "/templates/site.typ": site

#show: site.with(title: "About | Mira Chen", description: "About the fictional designer Mira Chen.", active: "about")

#html.main(class: "wrapper about-page")[
  #html.header(class: "page-hero")[
    #html.h1[About]
    #html.p(class: "tagline")[Thanks for stopping by. Read below to learn more about myself and my background.]
    #html.img(src: "/assets/studio.jpg", alt: "A quiet office lounge used for design reviews")
  ]
  #html.section(class: "about-section")[
    #html.h2[Background]
    #html.div[
      Mira is a fictional designer used by this example. Her practice combines
      interface design, frontend engineering, and research into tools that help
      people understand difficult systems.

      She works from prototypes outward: make the interaction concrete, test it
      with real tasks, and keep only the structure that earns its place.
    ]
  ]
  #html.section(class: "about-section")[
    #html.h2[Education]
    #html.div[Interaction design, information architecture, and computer science.]
  ]
  #html.section(class: "about-section")[
    #html.h2[Skills]
    #html.div[Creative development, product strategy, design systems, and technical writing.]
  ]
  #html.aside(class: "contact")[
    #html.h2[Interested in working together?]
    #html.a(class: "button-link", href: "mailto:me@example.com")[Send me a message #("->")]
  ]
]
