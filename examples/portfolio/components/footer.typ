#let footer() = html.footer(class: "site-footer")[
  #html.div[
    #html.p[Designed & developed in Portland with #link("https://astro.build/")[Astro].]
    #html.p[Copyright #datetime.today().display("[year]") Jeanine White]
  ]
  #html.p[
    #link("https://twitter.com/me")[Twitter]
    #link("https://github.com/me")[GitHub]
    #link("https://codepen.io/me")[CodePen]
  ]
]
