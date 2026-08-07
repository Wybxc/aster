#let footer() = html.footer[
  #html.p[Copyright #datetime.today().display("[year]")]
  #html.div[
    #html.a(href: "mailto:relay@example.com")[Contact]
    #html.a(href: "https://github.com/Wybxc/aster")[Source]
  ]
]
