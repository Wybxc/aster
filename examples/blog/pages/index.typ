#import "/lib.typ": date-label, date-value, post-url, posts
#import "/templates/site.typ": site

#let recent = posts().slice(0, calc.min(3, posts().len()))
#show: site.with(active: "home")

#html.main[
  #html.h1(class: "hero-title")[Notes for careful builders]
  #html.p[
    Field Notes is a small publication about learning, building, and the ideas
    that make software easier to understand.
  ]
  #html.p[
    Articles are written in Typst, collected automatically, and published as
    regular semantic HTML.
  ]

  #html.h2[Recent posts]
  #html.ul(class: "post-grid")[
    #for item in recent {
      let data = item.metadata
      html.li(class: "post-card")[
        #html.a(href: post-url(item.entry.id))[
          #html.img(src: data.image, alt: "")
          #html.h2[#data.title]
          #html.time(datetime: date-value(data.date))[#date-label(data.date)]
        ]
      ]
    }
  ]
]
