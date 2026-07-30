#metadata((
  title: "Content collections load entries only when pages need them.",
  section: "Guides",
  order: 20,
  summary: "This guide explains how lazy entry modules load metadata and content on demand.",
)) <frontmatter>

= Content collections load entries only when pages need them.

The adapter exports three queries:

- `get-collection-ids(name)` reads membership without loading entry bodies.
- `get-collection(name)` returns sorted entry modules.
- `get-entry(collection, id)` returns one entry module or `none`.

== Entry modules delay content evaluation.

An entry exposes `id`, `collection`, and `render`. It intentionally exposes no
file path and stores no evaluated content.

```typ
#let post = get-entry("journal", "incremental-by-default")
#let result = post.render()

#result.metadata.tags.join(", ")
#result.content
```

The render closure performs a normal Typst import. This preserves source-aware
diagnostics and lets comemo track the exact file dependency.
