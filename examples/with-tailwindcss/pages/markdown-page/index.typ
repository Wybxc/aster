#import "/templates/layout.typ": layout

#show: layout.with(title: "Typst page")

#html.main(class: "mx-auto max-w-2xl p-8")[
  #html.h1(class: "text-3xl font-bold")[Typst page]
  #html.p(class: "mt-4 text-slate-600")[This route uses Typst markup while sharing the same Tailwind CSS entry.]
  #html.a(class: "mt-6 inline-block underline", href: "/")[Back home]
]
