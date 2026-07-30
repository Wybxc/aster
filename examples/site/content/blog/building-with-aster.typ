= Building with Aster

This site is built with Aster. Here's what you need to get started.

== Project structure

```
project/
  aster.toml           # project configuration
  content/             # content collections
    blog/
      hello-world.typ
  lib/
    aster/
      content.typ      # public content API
  src/                 # page templates
    index.typ
  dist/                # build output
```

== Content collections

Entries live in `content/<collection>/.../<id>.typ`. Aster exposes them as lazy
modules whose `render` function imports the source only when called.

== Page templates

Files in `src/` become pages. They can:

- Query content with `#get-collection("blog")` and `#get-entry("blog", "id")`
- Generate routes without loading bodies using `#get-collection-ids("blog")`
- Load metadata and content with `#entry.render()`
- Use any Typst features for layout and design

#lorem(10)
