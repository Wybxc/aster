#import "/components/content.typ": callout, tabs

#metadata((
  title: "Authoring Content",
  description: "Use Typst markup and structured components to write browser documents.",
  section: "Guides",
  section_order: 20,
  order: 10,
)) <aster-frontmatter>

Pages and content entries are ordinary Typst modules compiled with Typst's HTML
feature enabled. Use markup where it improves readability and content functions
where structure is dynamic.

= Typst markup

Use *strong emphasis*, _emphasis_, `inline code`, lists, links, math such as
$e^(i pi) + 1 = 0$, and other Typst markup directly.

- Keep prose in markup where it is easiest to scan.
- Use content functions for dynamic headings, lists, and repeated UI.
- Use typed `html` elements when browser semantics matter.
- Use `html.elem` only when no typed element exists.

#callout(kind: "tip")[
  A content entry can import only the components it uses. Aster discovers their
  styles and scripts from `<aster-style>`, `<aster-script>`, and
  `<aster-module>` metadata in the rendered document.
]

= Complete HTML documents

A page template should return an HTML document. Aster can create a missing
`<head>` before `<body>` when it needs to attach generated styles, but an
explicit document remains clearest:

```typ
#let page(body) = html.html(lang: "en")[
  #html.head[
    #html.meta(charset: "utf-8")
    #html.title[Example]
    #html.link(rel: "stylesheet", href: "/styles/site.css")
  ]
  #html.body[
    #html.main[#body]
  ]
]

#show: page
```

= Code blocks

Fenced raw blocks retain their language identifier. Aster highlights supported
languages and publishes the generated Lumis theme stylesheet once per page.
Typst code and math use Aster's native Typst tokenizer; other supported grammars
are downloaded on demand and cached below the system cache directory.

#tabs("language-samples", (
  (
    label: "Typst",
    body: [
      ```typ
      #let greeting(name) = [Hello, #name!]
      #greeting("documentation")
      ```
    ],
  ),
  (
    label: "Rust",
    body: [
      ```rust
      #[derive(Clone)]
      struct Page { path: String }
      ```
    ],
  ),
))

= Links and resources

A leading slash in a navigation link refers to the generated site root. Aster
rewrites it relative to the current output page, which keeps the same build
deployable under a subdirectory.

Project resource references also begin with `/`, but resolve from the project
virtual root during the build. For example, `/assets/logo.svg` is published as a
content-addressed output asset.

#callout(kind: "note", title: "Two root-relative namespaces")[
  In navigational attributes such as `href`, `/guide/` denotes the site root. In
  resource attributes such as `src`, stylesheet links, or component metadata,
  `/assets/logo.svg` denotes the project virtual root. Aster resolves resources
  first and writes the correct page-relative output URL.
]
