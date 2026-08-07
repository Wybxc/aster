#import "/lib.typ": projects
#import "/components/project-grid.typ": project-grid
#import "/templates/site.typ": site

#let selected = projects().slice(0, calc.min(4, projects().len()))
#show: site.with(active: "home")

#html.div(class: "page-stack")[
  #html.div(class: "wrapper")[
    #html.header(class: "hero")[
      #html.div(class: "hero-copy")[
        #html.h1[Hello, my name is Jeanine White]
        #html.p(class: "tagline")[I am a Creative Developer who is currently based in Portland, Oregon.]
        #html.div(class: "roles")[
          #html.span(class: "pill")[Developer]
          #html.span(class: "pill")[Speaker]
          #html.span(class: "pill")[Writer]
        ]
      ]
      #html.img(src: "/assets/portrait.jpg", alt: "Jeanine White smiling in a red plaid shirt and glasses")
    ]
    #html.section(class: "skills")[
      #html.div[
        #html.span(class: "skill-icon", aria-hidden: true)[#("</>")]
        #html.h2[Full Stack]
        #html.p[Building thoughtful experiences from interface through infrastructure.]
      ]
      #html.div[
        #html.span(class: "skill-icon", aria-hidden: true)[#("*")]
        #html.h2[Industry Leader]
        #html.p[Sharing practical ideas through talks, writing, and collaboration.]
      ]
      #html.div[
        #html.span(class: "skill-icon", aria-hidden: true)[#("->")]
        #html.h2[Strategy-Minded]
        #html.p[Connecting product decisions to meaningful outcomes.]
      ]
    ]
  ]

  #html.main(class: "wrapper page-stack")[
    #html.section(class: "section")[
      #html.div(class: "section-heading")[
        #html.header(class: "section-header")[
          #html.h3[Selected Work]
          #html.p[Take a look at some of my featured work for clients from the past few years.]
        ]
        #html.a(class: "button-link", href: "/work/")[View all #("->")]
      ]
      #project-grid(selected)
    ]
    #html.section(class: "section")[
      #html.header(class: "section-header")[
        #html.h3[Mentions]
        #html.p[I have been fortunate to receive praise for my work in several publications.]
      ]
      #html.div(class: "mentions")[
        #for brand in ("Medium", "BuzzFeed", "The Next Web", "awwwards.", "TechCrunch") {
          html.span[#brand]
        }
      ]
    ]
    #html.aside(class: "contact")[
      #html.h2[Interested in working together?]
      #html.a(class: "button-link", href: "mailto:me@example.com")[Send me a message #("->")]
    ]
  ]
]
