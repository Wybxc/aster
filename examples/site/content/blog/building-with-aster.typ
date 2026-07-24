= Building with Aster

This site is built with Aster. Here's what you need to get started.

== Project structure

```
project/
  aster.toml           # project configuration
  content/             # content collections (compiled first)
    blog/
      hello-world.typ
  lib/
    aster/
      content.typ      # public content API
  src/                 # page templates (compiled second)
    index.typ
  dist/                # build output
```

== Content collections

Entries live in `content/<collection>/.../<id>.typ`. Each file is compiled
independently and its body text is extracted into a structured HTML tree.

== Page templates

Files in `src/` become pages. They can:

- Query content with `#get-collection("blog")` and `#get-entry("blog", "id")`
- Render entry bodies with `#render(entry)`
- Use any Typst features for layout and design

#lorem(10)
