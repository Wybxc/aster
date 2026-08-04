#import "@preview/example:0.1.0": add, mul
#import "/templates/site.typ": site

#let rows = (
  (
    source: [Typst evaluates #raw("range(4).map(x => x * x)").],
    result: [The result is #range(4).map(x => str(x * x)).join(", ").],
  ),
  (
    source: [The imported package evaluates #raw("add(8, 5) / mul(3, 4)").],
    result: [The results are #(str(add(8, 5)) + " and " + str(mul(3, 4)) + ".")],
  ),
  (
    source: [The source reads #raw("sys.inputs.site.edition") from the configuration.],
    result: [The configured value is #sys.inputs.site.edition.],
  ),
)
#let build-steps = (
  [Aster discovers every source file and content entry.],
  [Aster validates the complete route plan before rendering pages.],
  [A tracked Typst world compiles each page and records its dependencies.],
  [Aster publishes the generated HTML together with hashed assets.],
)

#show: site.with(
  title: "Typst can express the complete page.",
  root: "../",
  description: "This page shows how Aster preserves Typst evaluation, semantic output, layout, and source highlighting.",
)

#html.elem("header")[
  #heading(level: 1)[Typst can express the complete page.]
  #html.elem("p")[
    This static page shows how Aster preserves Typst evaluation, semantic
    document structure, layout output, and syntax-aware source code.
  ]
]

#html.elem("section")[
  #heading(level: 2)[Typst evaluates configuration, functions, and packages together.]
  Values from TOML, local expressions, and an imported package participate in
  the same evaluation that produces the document.
  #table(
    columns: (1.7fr, 1fr),
    table.header[
      *The source performs this work.*
    ][
      *The page displays this result.*
    ],
    ..rows.map(row => (
      [#row.source],
      [#row.result],
    )).flatten(),
  )
]

#html.elem("section")[
  #heading(level: 2)[Aster preserves semantic document output.]
  Typst emits semantic HTML for document elements and MathML for equations such
  as $integral_0^1 x^2 dif x = 1/3$, while Aster retains this structure during
  the remaining build stages.
  #enum(..build-steps)
]

#html.elem("section")[
  #heading(level: 2)[Typst can embed a laid-out frame in an HTML page.]
  `html.frame` runs Typst's layout engine and exports the result as an inline
  SVG, while the homepage separately demonstrates how Aster extracts a large
  embedded image into a hashed asset.
  #html.frame[
    #block(width: 270pt, fill: rgb("edf3fa"), inset: 16pt, radius: 4pt)[
      #set text(fill: rgb("203650"), size: 11pt)
      #set par(justify: false)
      #stack(
        dir: ttb,
        spacing: 4pt,
        [This SVG was laid out by Typst.],
        [Aster then embedded it in the HTML page.],
      )
    ]
  ]
]

#html.elem("section")[
  #heading(level: 2)[Aster applies syntax highlighting during document transformation.]
  Aster parses Typst-family code with Typst and highlights other languages with
  Syntect before combining the selected light and dark themes into one
  generated stylesheet.

  ```typ
  #metadata(
    get-collection-ids("docs").map(path => (path: path))
  ) <route>
  ```

  ```rust
  #[comemo::memoize]
  fn compile_page(world: Tracked<dyn World>) -> HtmlDocument {
      typst::compile(&*world)
  }
  ```
]
