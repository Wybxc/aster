#import "/components/content.typ": callout

#metadata((
  title: "Content Collections",
  description: "Discover entries by directory and load metadata or content only when needed.",
  section: "Guides",
  section_order: 20,
  order: 30,
)) <aster-frontmatter>

Aster discovers every `.typ` file below `content/`. The first path segment is
the collection name; the remaining path without `.typ` is the entry id.

For project-owned queries such as published posts, tags, pagination, and
archives, see the #link("/guides/recipes/")[Recipes] guide. The collection protocol stays
small so those policies can remain ordinary Typst code.

```text
content/posts/hello.typ          → collection "posts", id "hello"
content/docs/guides/start.typ    → collection "docs", id "guides/start"
```

A file directly under `content/` is invalid because every entry must belong to
a collection.

= Entry modules

The `_aster.collections` protocol maps collection names and ids to lazy entry
modules. Each entry exposes `id`, `collection`, `metadata()`, and `render()`.
The conventional `lib.typ` wraps lookup without hiding those entry modules:

```typ
#import "/lib.typ": get-entry

#let entry = get-entry("posts", "hello")
#if entry != none [
  #let data = entry.metadata()
  #html.h1[#data.title]
  #entry.render()
]
```

`metadata()` imports the entry and returns its `<aster-frontmatter>` metadata;
`render()` imports the same module and returns its content. Typst memoizes that
module evaluation when both are called in one build context.

= Writing an entry

```typ
#metadata((
  title: "Hello",
  date: "2026-08-12",
  draft: false,
)) <aster-frontmatter>

= First heading

The rest of the file is the rendered body.
```

Metadata is project-owned: Aster transports the value but does not impose a
schema. A page can sort and filter entries using any fields its templates
expect.

= Incremental behavior

Reading metadata or content records the imported source as a build dependency.
Pages that never access an entry do not depend on its body. Route probes should
use collection ids when only membership is needed:

```typ
#import "/lib.typ": get-collection-ids

#metadata(
  get-collection-ids("posts").map(id => (slug: id))
) <aster-route>
```

Adding, removing, or renaming an entry changes the shared collection manifest,
so pages that use the collection view are rebuilt.

#callout(kind: "note", title: "Editor evaluation")[
  Tinymist and standalone Typst do not inject `_aster`. Keep protocol helpers
  tolerant of a missing input, as this example's `lib.typ` does, so files remain
  analyzable outside an Aster build.
]
