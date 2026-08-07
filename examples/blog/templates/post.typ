#import "/lib.typ": date-label, date-value
#import "/templates/site.typ": site

#let post(body, item: (:)) = {
  let data = item.metadata
  show: site.with(title: data.title, description: data.description, active: "blog")
  html.main[
    #html.article(class: "blog-post")[
      #html.div(class: "hero")[
        #html.img(src: data.image, alt: "")
      ]
      #html.div(class: "post-title")[
        #html.time(datetime: date-value(data.date))[#date-label(data.date)]
        #html.h1[#data.title]
        #html.hr()
      ]
      #html.div(class: "prose")[#body]
      #html.p(class: "back-link")[#link("/blog/")[Back to all posts]]
    ]
  ]
}
