#import "/templates/site.typ": site

#show: site.with(
  title: "Page not found",
  description: "The requested page could not be found.",
  path: "/404.html",
)

#html.elem("main", attrs: (id: "main-content", class: "app-main not-found"))[
  #html.elem("p", attrs: (class: "error-code"))[404]
  #html.elem("h1")[Page not found]
  #html.elem("p")[The page may have moved, or the address may be incorrect.]
  #html.elem("a", attrs: (class: "primary-link", href: "/"))[Return home]
]
