#metadata((
  title: "Aster exposes configuration as Typst input.",
  section: "Reference",
  order: 40,
  summary: "This reference explains how TOML values become Typst inputs and select the syntax highlighting themes.",
)) <frontmatter>

= Aster exposes configuration as Typst input.

Aster parses `aster.toml` once and exposes the complete value through
`sys.inputs`. TOML tables become dictionaries and arrays of tables become Typst
arrays.

```toml
[site]
title = "Aster Field Notes"
edition = "2026"

[[navigation]]
label = "Overview"
href = "index.html"
```

Templates read the resulting values directly through `sys.inputs`:

```typ
#let site = sys.inputs.site
#html.title(site.title + " · " + site.edition)
```

The `[highlight.themes]` table selects built-in Syntect themes or project-local
`.tmTheme` files for generated light and dark syntax styles.
