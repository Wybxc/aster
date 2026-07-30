= Typst and HTML

Aster discovers each content entry and exposes a lazy entry module to page
templates through `sys.inputs._aster.collections`.

== How it works

The build combines a small Rust-owned manifest with Typst's module system:

*Content discovery.* Aster creates one module for each collection entry. The
module exposes `id`, `collection`, and a Typst `render` closure, but contains no
source path, content, or frontmatter values.

*Page compilation.* Files under `src/` import `lib/aster/content.typ` and use
`#get-collection()`, `#get-collection-ids()`, and `#get-entry()`. Calling an
entry's `#entry.render()` closure dynamically imports its source, includes its body,
and reads labelled frontmatter with ordinary Typst code. Route metadata can use
`#get-collection-ids()` without loading those bodies.

== The content protocol

The data flowing between phases uses a simple protocol:

```typc
let protocol = 3
let posts = (
  blog: (
    hello-world: entry-module,
  ),
)
```

This page template wraps each post in an `<article>` element and renders the
body from `#post.render().content`.
