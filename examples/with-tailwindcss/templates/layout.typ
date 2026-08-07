#let layout(body, title: "Astro + Tailwind CSS") = html.html(lang: "en")[
  #html.head[
    #html.meta(charset: "utf-8")
    #html.meta(name: "viewport", content: "width=device-width")
    #html.meta(name: "generator", content: "Aster")
    #html.link(rel: "icon", type: "image/svg+xml", href: "/assets/favicon.svg")
    #html.title(title)
    #html.elem("link", attrs: (rel: "tailwind", href: "/styles/global.css"))
  ]
  #html.body[#body]
]
