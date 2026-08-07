#metadata((
  title: "Composing articles with content functions",
  description: "Mix reusable Typst components with article markup.",
  date: "2024-06-01",
  image: "/assets/cover-components.jpg",
)) <aster-frontmatter>

Aster uses Typst content functions to place reusable components directly inside
an article without introducing a second content language.

= Why content functions?

Typst markup and scripting share one document. A component can accept content,
produce semantic HTML, and attach its own styles or scripts.

#let callout(body) = html.aside(
  style: "border-left: 4px solid #2337ff; padding: 1rem; background: #f4f5ff",
)[#body]

#callout[
  *Embedded component:* this panel is declared and rendered directly inside
  the article source.
]

= Example

```typ
#let callout(body) = html.aside(class: "callout")[#body]

#callout[Reusable content lives here.]
```

This remains static HTML by default. Components can also declare page resources
with Aster's labeled style and script metadata when interaction is needed.
