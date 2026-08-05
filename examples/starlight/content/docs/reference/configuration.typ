#import "/components/content.typ": aside, doc-heading

#metadata((
  title: "Configuration Reference",
  description: "Configure project layout, output processing, and site-owned values.",
  section: "Reference",
  section_order: 30,
  order: 20,
  toc: (
    (id: "manifest", title: "Project manifest", level: 2),
    (id: "build-settings", title: "Build settings", level: 2),
    (id: "site-settings", title: "Site settings", level: 2),
  ),
)) <aster-frontmatter>

The `aster.toml` manifest has two readers. Aster deserializes build-owned tables,
while Typst templates read project-specific values directly with `toml`.

#doc-heading(id: "manifest")[Project manifest]

```toml
[project]
name = "aster-starlight"

[site]
title = "Aster Docs"
language = "en"
```

#doc-heading(id: "build-settings")[Build settings]

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

#aside(kind: "note")[
  These defaults are omitted from this example manifest. Configuration remains
  focused on values that differ from Aster's conventional behavior.
]

#doc-heading(id: "site-settings")[Site settings]

Tables unknown to Aster remain available to Typst. The template reads the site
title, language, repository URL, and edit-link base from the same TOML file, so
there is no second injected configuration dictionary.
