#import "/lib.typ": date-label, date-value

#let release(item, body, linked: true, single: false) = {
  let data = item.metadata
  html.article(class: "release" + if single { " single" } else { "" })[
    #html.div(class: "version-wrapper")[
      #html.div(class: "version-info")[
        #if linked {
          html.a(href: "/releases/" + item.entry.id + "/")[
            #html.span(class: "version-number")[#data.version]
            #html.time(class: "date", datetime: date-value(data.date))[#date-label(data.date)]
          ]
        } else [
          #html.span(class: "version-number")[#data.version]
          #html.time(class: "date", datetime: date-value(data.date))[#date-label(data.date)]
        ]
      ]
    ]
    #html.div(class: "content")[#body]
  ]
}
