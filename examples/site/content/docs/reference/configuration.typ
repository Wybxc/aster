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

== Build configuration

The Aster-owned tables control project layout and output processing. All paths
are relative to the project root. Source, content, output, and configured font
directories must not overlap the output tree.

```toml
[paths]
source = "src"
content = "content"
output = "dist"

[output]
assets = "_assets"
pretty = false

[assets]
image-inline-threshold = 1024
minify-css = true

[typst.fonts]
paths = ["fonts"]
system = true

[highlight]
enabled = true
themes = { light = "InspiredGitHub", dark = "base16-eighties.dark" }
```

`image-inline-threshold` is the decoded byte size at which an image data URL
becomes a generated asset. Setting `system` to `false` makes font discovery
depend only on the configured project-local font directories.
