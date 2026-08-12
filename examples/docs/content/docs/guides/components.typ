#import "/components/content.typ": callout, card-grid, steps

#metadata((
  title: "Components",
  description: "Compose reusable browser UI from Typst content functions.",
  section: "Guides",
  section_order: 20,
  order: 20,
  badge: "UI",
)) <aster-frontmatter>

Components accept structured values and return content. Their browser resources
can remain next to the rendering function and are deduplicated per page.

= Component APIs

#card-grid((
  (title: "Content", body: [Write prose with Typst markup and return content from functions.]),
  (title: "Structure", body: [Use semantic typed HTML elements for browser meaning.]),
  (title: "Resources", body: [Attach CSS, classic scripts, or modules through labelled metadata.]),
))

#steps((
  (title: "Define the API", body: [Choose explicit arguments that encode the component invariant.]),
  (title: "Return content", body: [Build semantic HTML and use classes only when element meaning is insufficient.]),
  (title: "Attach resources", body: [Keep component-owned behavior in the same project source module.]),
))

= Resource declarations

A declaration contains a project path string or exactly one fenced raw block
plus surrounding whitespace:

````typ
#let counter(body) = [
  #metadata(
    ```css
    .counter { display: inline-flex; gap: .5rem; }
    ```
  ) <aster-style>
  #metadata("./counter.js") <aster-script>
  #html.div(class: "counter")[#body]
]
````

Use `<aster-style>` for CSS, `<aster-script>` for a classic script loaded from
the document head with `defer`, and `<aster-module>` for an ES module bundled by
`esbuild` and loaded with `type="module"`.

Relative paths resolve from the Typst source file containing the declaration;
paths beginning with `/` resolve from the project virtual root. Inline resources
are extracted into generated files. Reusing a component on one page does not
duplicate its declarations, while declarations from different components remain
distinct and retain document order.

#callout(kind: "caution")[
  Managed component resources currently need to originate in project files.
  Resources declared by Typst packages cannot be published yet.
]

= This site's components

#callout(kind: "note")[Use notes for supporting context.]
#callout(kind: "tip")[Use tips for optional improvements.]
#callout(kind: "caution")[Use cautions for surprising behavior.]
#callout(kind: "danger")[Use danger notices for destructive actions.]
