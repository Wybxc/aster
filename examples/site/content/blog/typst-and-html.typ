= Typst and HTML

Aster compiles each content entry into an HTML structure, then makes it
available to your page templates through `sys.inputs._aster.collections`.

== How it works

The build happens in two phases:

*Phase 1: content collection.* Every `.typ` file under `content/` is compiled
to an `HtmlDocument`, the `<body>` children are extracted, and the resulting
DOM tree is converted into a Typst dictionary that gets passed along as
`sys.inputs._aster.collections`.

*Phase 2: page compilation.* Files under `src/` are compiled into pages. These
pages can import `lib/aster/content.typ` and use `#get-collection()`,
`#get-entry()`, and `#render()` to query and display content.

== The content protocol

The data flowing between phases uses a simple protocol:

```typc
let protocol = 1
let posts = (
  blog: (
    hello-world: (
      id: "hello-world",
      body: (
        (kind: "element", tag: "h2", attrs: (:), children: (
          (kind: "text", value: "Hello, Aster!"),
        )),
      ),
    ),
  ),
)
```

This page template wraps each post in an `<article>` element and renders the
body with `#render()`.
