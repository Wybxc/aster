#import "/templates/site.typ": site

#show: site.with(title: "Page not found | Jeanine White", description: "The requested page was not found.")

#html.main(class: "wrapper not-found")[
  #html.div[
    #html.h1[404]
    #html.p[This page wandered off. #link("/")[Return home].]
  ]
]
