#import "/components/content.typ": aside, tabs

#metadata((
  title: "Authoring Content",
  description: "Use Typst markup and structured components to write documentation.",
  section: "Guides",
  section_order: 20,
  order: 10,
)) <aster-frontmatter>

Documentation entries are ordinary Typst modules. Each entry exposes metadata
for navigation and renders its content only when a route needs it.

= Typst markup

Use *strong emphasis*, _emphasis_, `inline code`, lists, links, math such as
$e^(i pi) + 1 = 0$, and other Typst markup directly.

- Keep prose in markup where it is easiest to scan.
- Use content functions for dynamic headings, tabs, and repeated UI.
- Use typed `html` elements when browser semantics matter.

#aside(kind: "tip")[
  A content entry can import only the components it uses. Aster discovers their
  styles and scripts from `<aster-style>` and `<aster-script>` metadata.
]

= Code blocks

Fenced raw blocks retain their language identifier. Aster highlights supported
languages and publishes the generated theme stylesheet once.

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
      fn main() {
          println!("static output");
      }
      ```
    ],
  ),
))

= Links and assets

A leading slash in a navigation link refers to the generated site root. Aster
rewrites it relative to the current output page, which keeps the same build
deployable under a subdirectory.

Project resource references also begin with `/`, but resolve from the project
virtual root during the build. For example, `/assets/logo.svg` is published as a
content-addressed output asset.
