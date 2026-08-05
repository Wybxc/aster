#let page-header(title, description) = [
  #metadata("./page-header.css") <aster-style>
  #html.elem("header")[
    #html.elem("h1")[#title]
    #html.elem("p")[#description]
  ]
]
