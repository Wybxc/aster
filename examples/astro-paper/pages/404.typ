#import "/templates/site.typ": site

#show: site.with(
  title: "Page not found",
  description: "The requested page could not be found.",
  path: "/404.html",
)

#metadata("./404.css") <aster-style>
#html.main(id: "main-content")[
  #html.p[404]
  #html.h1[Page not found]

  The page may have moved, or the address may be incorrect.

  #html.a(href: "/")[Return home]
]
