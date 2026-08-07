#import "/lib.typ": date-label, date-value, post-url, posts
#import "/templates/site.typ": site

#let recent = posts().slice(0, calc.min(3, posts().len()))
#show: site.with(active: "home")

#html.main[
  #html.h1(class: "hero-title")[🧑‍🚀 Hello, Astronaut!]
  #html.p[
    Welcome to my corner of the internet. I write about learning, building,
    and the small ideas that make software more enjoyable.
  ]
  #html.p[
    This starter keeps the approachable shape of Astro's official blog example,
    while its pages and content are authored in Typst.
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
