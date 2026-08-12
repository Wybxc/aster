#import "/components/content.typ": callout

#metadata((
  title: "Runtime Protocol",
  description: "Read Aster's reserved Typst input for collections, routes, and rendered pages.",
  section: "Reference",
  section_order: 30,
  order: 30,
)) <aster-frontmatter>

Aster reserves `sys.inputs._aster` for build context. The current protocol value
has this shape:

```typc
(
  protocol: 7,
  version: "0.1.0",
  collections: (:),
  route: none,
  routes: (pages: ()),
  site: (pages: ()),
)
```

`protocol` is the compatibility version, while `version` is the Aster package
version. Project code should validate the protocol version when `_aster` exists
and remain usable when the entire input is absent during editor evaluation.

= Collections

`collections` maps a collection name to a dictionary of lazy entry modules.
Each module exposes:

- `id`: the nested id without `.typ`;
- `collection`: its first directory below `content/`;
- `metadata()`: the entry's `<aster-frontmatter>` dictionary, or an empty dictionary;
- `render()`: the entry's rendered Typst content.

= Route phases

`route` is `none` in the base runtime used to probe dynamic templates. Concrete
page and generator compilations receive `(path, params)`. `routes.pages` is
added after page planning and remains available while pages and generators run.

= Rendered site

Generators run after page transformation and receive `site.pages`. Each item is:

```typc
(
  path: "/guide/",
  html: "<!DOCTYPE html>...",
  content: (
    html: "<article>...</article>",
    text: "Readable article text",
  ),
)
```

`html` is the final encoded page before external postprocessing. `content` is
`none` unless the page has one element labelled `<aster-content>`. That label
selects the main fragment used by feeds or indexes and is removed from final
HTML. More than one labelled content root is an error.

```typ
#html.article[
  #entry.render()
] <aster-content>
```

#callout(kind: "note")[
  The protocol is build context, not project configuration. Read site-owned
  settings with `toml("/aster.toml")` rather than adding parallel Typst inputs.
]
