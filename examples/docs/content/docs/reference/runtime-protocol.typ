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
  protocol: 9,
  version: "0.1.0",
  collections: (:),
  route: module,
  routes: module,
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

`route` is a stable module with native `path(default: none)` and
`param(name, default: none)` functions. During a dynamic probe they return the
provided default. A concrete page or generator compilation supplies their
values through its Typst world, so changing routes does not require rebuilding
the shared library. The stable `routes` module exposes `pages()`, which returns
an empty array during route discovery and the complete planned page URL set
while pages and generators run. Supplying that set through the Typst world also
does not rebuild the library.

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
