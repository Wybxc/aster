#metadata((
  title: "Using content components",
  description: "Mix reusable Typst components with article markup.",
  date: "2024-06-01",
  image: "/assets/blog-placeholder-5.jpg",
)) <aster-frontmatter>

Astro's original example uses MDX to mix components into Markdown. In Aster,
Typst content functions provide the same composition model without a separate
content language.

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
