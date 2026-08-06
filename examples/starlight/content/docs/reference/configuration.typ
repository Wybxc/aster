#import "/components/content.typ": callout

#metadata((
  title: "Configuration Reference",
  description: "Configure project layout, output processing, and site-owned values.",
  section: "Reference",
  section_order: 30,
  order: 20,
)) <aster-frontmatter>

The `aster.toml` manifest has two readers. Aster deserializes build-owned tables,
while Typst templates read project-specific values directly with `toml`.

= Project manifest

```toml
[project]
name = "aster-starlight"

[site]
title = "Aster Docs"
language = "en"
```

= Build settings

The standard path layout uses `pages`, `content`, `public`, and `dist`. CSS is
processed through Lightning CSS and generated assets are written below
`dist/_assets` unless configured otherwise.

```toml
[css]
minify = true
targets = ["defaults"]

[highlight]
themes = { light = "InspiredGitHub", dark = "base16-eighties.dark" }
```

#callout(kind: "note")[
  These defaults are omitted from this example manifest. Configuration remains
  focused on values that differ from Aster's conventional behavior.
]

= Site settings

Tables unknown to Aster remain available to Typst. The template reads the site
title, language, repository URL, and edit-link base from the same TOML file, so
there is no second injected configuration dictionary.
