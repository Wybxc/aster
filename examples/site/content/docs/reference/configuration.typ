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
href = "./"
```

Templates read the resulting values directly through `sys.inputs`:

```typ
#let site = sys.inputs.site
#html.title(site.title + " · " + site.edition)
```

The `[highlight.themes]` table selects built-in Syntect themes or project-local
`.tmTheme` files for generated light and dark syntax styles.

== Build configuration

The Aster-owned tables control project layout and output processing. Paths in
`[paths]` and configured font paths are relative to the project root. Pages,
content, public, and configured font directories must not overlap the output
tree. `[output].assets` is relative to that output tree and names Aster's
generated asset directory, not the project's optional `assets/` source folder.

```toml
[paths]
pages = "pages"
content = "content"
public = "public"
output = "dist"

[output]
assets = "_assets"
pretty = false

[assets]
image-inline-threshold = 1024

[css]
minify = true
targets = ["defaults"]
custom-media = false

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

CSS files linked with `rel=\"css\"` are bundled with Lightning CSS. `targets`
accepts Browserslist queries and controls syntax lowering and vendor prefixes.
When it is omitted, Aster preserves modern syntax rather than assuming a
browser support policy. `custom-media` enables the draft `@custom-media`
syntax; `minify` only controls output compression and does not disable target
transforms. A leading `/` in a `rel=\"css\"` link resolves from the project
virtual root, so `/styles/site.css` selects that project file. Inside CSS,
`@import` and `url()` keep standard URL semantics: relative references are
bundled from the current stylesheet, while `/...` remains a website-root URL.
