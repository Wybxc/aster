#import "/templates/site.typ": site

#show: site.with(title: "About | Jeanine White", description: "About Jeanine White.", active: "about")

#html.main(class: "wrapper about-page")[
  #html.header(class: "page-hero")[
    #html.h1[About]
    #html.p(class: "tagline")[Thanks for stopping by. Read below to learn more about myself and my background.]
    #html.img(src: "/assets/at-work.jpg", alt: "Jeanine White at work with a colleague")
  ]
  #html.section(class: "about-section")[
    #html.h2[Background]
    #html.div[
      Lorem ipsum dolor sit amet, #link("https://astro.build/")[Astro] makes
      people happy. Sed do eiusmod tempor incididunt ut labore et dolore magna
      aliqua. Proin nibh nisl condimentum id venenatis a condimentum vitae.

      Arcu dui vivamus arcu felis bibendum ut tristique et egestas. Eget gravida
      cum sociis natoque penatibus. Porta nibh venenatis cras sed felis eget.
    ]
  ]
  #html.section(class: "about-section")[
    #html.h2[Education]
    #html.div[Corporis voluptates tenetur laudantium.]
  ]
  #html.section(class: "about-section")[
    #html.h2[Skills]
    #html.div[Creative development, product strategy, design systems, and technical writing.]
  ]
  #html.aside(class: "contact")[
    #html.h2[Interested in working together?]
    #html.a(class: "button-link", href: "mailto:me@example.com")[Send me a message #("->")]
  ]
]
