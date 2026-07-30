#metadata((
  title: "Type Atlas",
  year: "2025",
  slug: "type-atlas",
  status: "archive",
  summary: "Type Atlas generates a compact reference site from nested Typst content entries.",
  stack: ("Typst", "HTML", "CSS"),
)) <frontmatter>

= Type Atlas

Type Atlas is a small reference publishing experiment. Its sections are stored
as nested content ids and emitted through one spread route.

== Type Atlas demonstrates how nested publishing works.

#let capabilities = (
  [Aster returns collection entries in a stable order.],
  [Nested ids preserve the source hierarchy in the generated site.],
  [A document show rule supplies the shared HTML structure.],
  [Typst headings, lists, and tables become semantic HTML.],
)

#enum(..capabilities)

This archived project is included to make the project collection span multiple
years and route parameter combinations.
