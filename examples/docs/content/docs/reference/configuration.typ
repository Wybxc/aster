#import "/components/content.typ": callout

#metadata((
  title: "Configuration Reference",
  description: "Configure project layout, output transforms, watching, fonts, and postprocessors.",
  section: "Reference",
  section_order: 30,
  order: 20,
)) <aster-frontmatter>

`aster.toml` has two readers. Aster deserializes the build-owned tables listed
below. Typst code reads the complete manifest directly with
`toml("/aster.toml")`, including arbitrary project-owned tables such as
`[site]`, `[navigation]`, or `[author]`. Aster does not inject manifest values
into `sys.inputs`.

= Defaults

The following manifest shows every Aster-owned option and its default value:

```toml
[paths]
pages = "pages"
generate = "generate"
content = "content"
public = "public"
output = "dist"

[output]
assets = "_assets"
pretty = false

[assets]
image-inline-threshold = 1024

[assets.images]
enabled = true
jpeg-quality = 85
# frame-density is unset by default

[css]
minify = true
targets = ["defaults"]
custom-media = false

[watch]
paths = []

[typst.fonts]
paths = []
system = true

[highlight]
enabled = true

[highlight.themes]
light = "github_light"
dark = "github_dark"
```

All project paths are relative to the project root. The five directories under
`[paths]` must be pairwise disjoint and cannot be the project root itself. Font
and watch paths must not overlap the output tree; a watch path also cannot be the
project root.

= Output and assets

`output.assets` names the generated-asset directory inside the output tree.
`output.pretty` controls HTML indentation only. CSS minification is controlled
separately by `css.minify`.

`image-inline-threshold` is measured after decoding a data URL. Content smaller
than the threshold remains inline; content at or above it is published.
`frame-density` is an optional pixel-density multiplier for raster images inside
Typst `html.frame` output.

`css.targets` accepts Browserslist queries. An empty array selects Lightning
CSS's default target set; the Aster default is the Browserslist query
`"defaults"`. `custom-media` enables parsing and transforming `@custom-media`.

= Fonts and highlighting

Font paths are scanned recursively. When `system` is true, Aster also discovers
installed system fonts. Typst's embedded fonts, including New Computer Modern
Math, remain available independently of that setting.

Highlight theme values are Lumis built-in theme names or project-relative theme
files. A theme load failure produces one build warning and leaves code
unhighlighted rather than failing the site build.

= Additional watch paths

```toml
[watch]
paths = ["data", "tailwind.config.js"]
```

Existing directories are watched recursively, files non-recursively, and
missing paths remain dependencies until created. Use this list for inputs read
by external tools that Aster cannot discover itself, especially Tailwind source
files outside the stylesheet entry's dependency graph.

= Postprocessors

```toml
[[postprocess]]
name = "search"
command = ["search-indexer", "--site", "{site}", "--output", "{output}"]
mount = "search"
watch = ["search.config.json"]
```

`command` is an executable followed by arguments and runs from the project root
without an implicit shell. An argument exactly equal to `{site}` receives the
mutable staging tree. `{output}` creates a private temporary output directory;
when it is used, `mount` is required and imports that tree below the given output
path. Without `{output}`, `mount` must be absent. `watch` adds project-relative
inputs for development commands.

#callout(kind: "note")[
  Project-owned top-level tables remain intentionally open. Aster-owned nested
  sections may reject unknown fields so misspelled build options do not silently
  become project data.
]
