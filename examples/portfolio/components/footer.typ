#let footer() = html.footer(class: "site-footer")[
  #html.div[
    #html.p[Designed in Vancouver and built with #link("https://github.com/Wybxc/aster")[Aster].]
    #html.p[Copyright #datetime.today().display("[year]") Mira Chen]
  ]
  #html.p[
    #link("https://github.com/Wybxc/aster")[Source]
    #link("mailto:mira@example.com")[Email]
  ]
]
