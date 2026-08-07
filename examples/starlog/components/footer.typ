#let footer() = html.footer[
  #html.p[Copyright #datetime.today().display("[year]")]
  #html.div[
    #html.a(href: "#")[Discord]
    #html.a(href: "#")[X]
    #html.a(href: "#")[GitHub]
  ]
]
