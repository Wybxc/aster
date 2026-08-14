#import "/components/content.typ": callout

#metadata((
  title: "Assets and Processing",
  description: "Publish project resources, bundle CSS and scripts, and optimize raster images.",
  section: "Guides",
  section_order: 20,
  order: 50,
)) <aster-frontmatter>

Aster transforms resources referenced by a compiled HTML page and publishes
generated files below `output.assets`, `_assets` by default. File names retain a
source-related stem when available and include a content hash, so repeated
publication of the same resource is deduplicated and unchanged output can be
cached safely.

= URL classification

- `/assets/logo.svg` resolves from the project virtual root.
- `./logo.svg` resolves relative to the source that contains it.
- Protocol URLs, `//` references, fragments, and query-only references stay browser-managed.
- `data:` URLs remain inline below the configured threshold and are extracted at or above it.

Query strings and fragments on local files are preserved after publication.
Project paths are constrained lexically; filesystem reads follow symbolic links.

= HTML resources

Aster publishes common image, media, embedded-object, icon, manifest, preload,
download, SVG-reference, and social-image attributes. `srcset` and
`imagesrcset` candidates are processed individually. Ordinary navigation links
are rewritten relative to the current output page rather than published as
files.

= CSS

```typ
#html.link(rel: "stylesheet", href: "/styles/site.css")
```

Lightning CSS bundles `@import` rules, rewrites local `url()` dependencies,
applies configured browser targets and custom-media transforms, and serializes
the result. `rel: "tailwind"` first runs the external Tailwind CSS v4 CLI, then
passes its output through the same Lightning CSS pipeline.

Tailwind source discovery outside the CSS entry and its configuration is an
external concern; list additional inputs under `[watch].paths`.

= Scripts

File-backed classic scripts are published without bundling; an inline classic
HTML script stays inline, while an inline `<aster-script>` declaration is
extracted to a file. ES modules declared with `<aster-module>` or local
`<script type="module">` elements are bundled for the browser by the external
`esbuild` CLI. Aster reads the esbuild metafile so module dependencies
participate in incremental rebuilds. HTTP imports remain external.

= Images

When optimization is enabled, Aster losslessly optimizes PNG files and
re-encodes JPEG files at `jpeg-quality`. Explicit HTML `width` and `height`
values provide a bounding box for downsampling without changing aspect ratio or
upscaling. GIF and WebP resources are left unchanged.

Typst `html.frame` remains an inline SVG. Raster images inside the frame can be
optimized in place; `frame-density` additionally limits their pixel dimensions
according to rendered size. The frame is not replaced with an `<img>` element.

#callout(kind: "note")[
  `public/` has different semantics: its files are copied byte-for-byte to the
  output root, keep their names, and are not processed or content-addressed.
  Use it for host-level files that must have an exact name.
]
