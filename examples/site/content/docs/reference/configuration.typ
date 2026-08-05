#metadata((
  title: "Aster exposes configuration as Typst input.",
  section: "Reference",
  order: 40,
  summary: "This reference explains how TOML values become Typst inputs and select the syntax highlighting themes.",
)) <aster-frontmatter>

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

[watch]
paths = []

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

CSS entries are ultimately bundled and transformed with Lightning CSS. `targets`
accepts Browserslist queries and controls syntax lowering and vendor prefixes.
It defaults to the Browserslist `defaults` query; set it to an empty list to
preserve modern syntax without assuming a browser support policy. `custom-media`
enables the draft `@custom-media` syntax; `minify` only controls whether the
serialized output retains unnecessary whitespace and does not disable target
transforms. A standard `rel=\"stylesheet\"` link goes directly through Lightning
CSS. A link with `rel=\"tailwind\"` first runs its source through the
external `tailwindcss` CLI and then uses the same Lightning CSS transforms and
asset publication. The Tailwind relation becomes `rel=\"stylesheet\"` in the
published HTML. The CLI must be installed separately and available on `PATH`; Aster invokes
it once per Tailwind entry on each build, while `aster dev` and `aster watch`
remain responsible for watching source files.
Project-local pages and content are tracked by the normal build, and Aster also
watches each Tailwind CSS entry. Config loaded through `@config`, local CSS
imports, and any other sources read only by Tailwind must be declared through
`watch.paths`. Each entry is a project-relative file or directory. Directories
are watched recursively; missing paths are retained and classified after they
are created. The project root and paths overlapping the output directory are
rejected to prevent rebuild loops.
A leading `/` in a resource reference resolves from the project virtual root, so
`/styles/site.css` selects that project file. This applies to stylesheet links,
scripts, ordinary HTML assets, and CSS `@import` and `url()` dependencies.
Protocol URLs and `//` references remain browser-managed.
