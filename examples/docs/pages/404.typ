#let settings = toml("/aster.toml")

#html.html(lang: settings.site.language)[
  #html.head[
    #html.meta(charset: "utf-8")
    #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
    #html.meta(name: "description", content: "The requested documentation page was not found.")
    #html.title[Page not found | #settings.site.title]
    #html.link(rel: "icon", type: "image/svg+xml", href: "/assets/logo.svg")
    #html.script(src: "/scripts/theme-init.js")
    #html.link(rel: "stylesheet", href: "/styles/base.css")
  ]
  #html.body[
    #html.main(style: "display:grid;min-height:100vh;place-items:center;padding:2rem;text-align:center")[
      #html.div[
        #html.img(src: "/assets/logo.svg", alt: "", width: 64, height: 64, style: "margin:0 auto 1.5rem")
        #html.h1(style: "color:var(--sl-color-white);font-size:2.625rem")[Page not found]
        #html.p(style: "margin-top:.75rem")[The page may have moved or no longer exists.]
        #html.p(style: "margin-top:1.5rem")[#html.a(href: "/")[Return to the documentation]]
      ]
    ]
  ]
]
