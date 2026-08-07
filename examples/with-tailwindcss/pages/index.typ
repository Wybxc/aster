#import "/components/button.typ": button
#import "/templates/layout.typ": layout

#show: layout

#html.main(class: "grid min-h-screen place-items-center content-center gap-2")[
  #button[Tailwind Button in Aster!]
  #html.a(class: "p-4 underline", href: "/markdown-page/")[Typst markup is also supported...]
]
