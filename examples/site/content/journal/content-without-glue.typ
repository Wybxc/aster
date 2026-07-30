#metadata((
  title: "Collections remain ordinary Typst files.",
  date: "2026-07-18",
  summary: "Aster adds a lazy Rust module boundary without changing the Typst files stored in a collection.",
  tags: ("content", "collections", "routing"),
)) <frontmatter>

= Collections remain ordinary Typst files.

Every `.typ` file below `content/<collection>/` becomes an entry. Nested paths
become nested ids, which makes a documentation tree a natural content
collection rather than a separate subsystem.

```typ
#import "/lib/aster/content.typ": get-entry

#let entry = get-entry("journal", "content-without-glue")
#let rendered = entry.render()

#rendered.metadata.title
#rendered.content
```

`get-entry` returns the module stored in `sys.inputs` unchanged. Calling
`render` dynamically imports only that source file and returns a dictionary
containing `metadata` and `content`.

== One collection model supports different routes.

The journal uses a normal `[slug]` route. Projects use two parameters in one
filename, while documentation uses a `[...path]` spread route for nested ids.
All three are declared with ordinary Typst metadata.
