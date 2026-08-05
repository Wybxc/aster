#import "/templates/site.typ": site

#show: site.with(
  title: "Page not found",
  description: "The requested page could not be found.",
  path: "/404.html",
)

#metadata("./404.css") <aster-style>
#html.elem("main", attrs: (id: "main-content"))[
  #html.elem("p")[404]
  #html.elem("h1")[Page not found]
  #html.elem("p")[The page may have moved, or the address may be incorrect.]
  #html.elem("a", attrs: (href: "/"))[Return home]
]
