#import "/lib.typ": date-label, date-value, post-url, posts
#import "/templates/site.typ": site

#show: site.with(title: "Blog", description: "All published articles.", active: "blog")

#html.main[
  #html.h1[Blog]
  #html.p[Notes, guides, and updates in reverse chronological order.]
  #html.ul(class: "post-grid")[
    #for item in posts() {
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
