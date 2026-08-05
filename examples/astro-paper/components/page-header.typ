#let page-header(title, description) = [
  #metadata("./page-header.css") <aster-style>
  #html.header(class: "page-header")[
    #html.h1[#title]
    #html.p[#description]
  ]
]
