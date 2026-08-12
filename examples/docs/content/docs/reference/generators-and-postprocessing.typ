#import "/components/content.typ": callout

#metadata((
  title: "Generators and Postprocessing",
  description: "Produce exact-path files in Typst and run external tools against the staged site.",
  section: "Reference",
  section_order: 30,
  order: 40,
)) <aster-frontmatter>

Generators and postprocessors extend different phases. A generator is a Typst
program that creates one declared file from build context. A postprocessor is an
explicit external command that runs after the complete site has been staged.

= Generators

Every `.typ` file under `generate/` maps to the same relative output path with
only its final `.typ` extension removed:

```text
generate/atom.xml.typ          → /atom.xml
generate/search/index.json.typ → /search/index.json
```

A concrete generator compilation must produce exactly one
`<aster-output>` metadata value, and that value must be a string or bytes.

```typ
#let pages = sys.inputs._aster.site.pages
#let paths = pages.map(page => page.path).join("\n")
#metadata(paths) <aster-output>
```

Generators run after every page has been transformed and encoded. They can read
`_aster.site.pages` to build Atom feeds, sitemaps, search documents, or arbitrary
project-owned formats. A labelled `<aster-content>` page fragment provides both
final HTML and plain text for feed bodies.

Dynamic generators use bracket parameters and `<aster-route>` exactly like
dynamic pages. Their route probe occurs after rendered pages are available, so
their declared parameter sets can depend on the site snapshot. Generator outputs
must not collide with pages, public files, assets, or one another.

= Postprocessors

Postprocessors are configured with `[[postprocess]]` entries and run in manifest
order against the unpublished staging tree. They have two integration modes:

- `{site}` lets a command inspect or mutate the complete staged site directly.
- `{output}` gives it a private directory whose files are imported below `mount`.

```toml
[[postprocess]]
name = "index"
command = ["site-index", "{site}", "{output}"]
mount = "search"
watch = ["indexer.toml"]
```

Commands run directly from the project root, without a shell. A nonzero exit,
missing output directory, invalid path, or collision fails publication and
preserves the previous output tree.

#callout(kind: "tip", title: "Choose the narrowest phase")[
  Prefer a generator when Typst and the rendered page snapshot are sufficient.
  Use a postprocessor for external programs that need the complete filesystem
  tree, such as a search indexer or deployment-specific optimizer.
]
