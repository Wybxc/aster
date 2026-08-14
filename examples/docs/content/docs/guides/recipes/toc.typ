#import "/components/content.typ": callout

#metadata((
  title: "Recipe: Heading IDs and Tables of Contents",
  description: "Collect headings from rendered Typst content and render a page TOC.",
  section: "Guides",
  section_order: 20,
  order: 42,
)) <aster-frontmatter>

Heading ids and TOC entries are page-template policy, not content collection
behavior. The docs example applies a show rule around `entry.render()`:

```typ
#import "/components/content.typ": heading-element, heading-id, heading-text

#let with-heading-rules(body) = {
  show heading: it => {
    let id = heading-id(heading-text(it.body))
    heading-element(it.body, id, it.level + 1)
  }
  body
}
```

`heading-text` extracts readable text from markup, while `heading-id` removes
unsupported characters and normalizes separators. The id allocator must also
make repeated headings unique and provide a fallback for headings with no
usable text.

= Collect heading metadata

The heading element receives a project-specific metadata label. A `context`
function can query all labels after the body has been evaluated and pass
`(id, title, level)` records to the TOC renderer:

```typ
#let collect-toc(use-toc) = context {
  use-toc(query(<aster-doc-toc-heading>).map(heading => (
    id: heading.attrs.at("id", default: ""),
    title: heading-text(heading.body),
    level: int(heading.tag.slice(1)),
  )))
}
```

Use a namespaced label so content authors cannot accidentally collide with the
recipe's query. The query observes the complete document regardless of where
the heading was created.

= Render the TOC

The collector should pass data to a separate renderer:

```typ
#let toc-list(items) = html.ul[
  #for item in items {
    html.li(class: "toc-level-" + str(item.level))[
      #html.a(href: "#" + item.id)[#item.title]
    ]
  }
]

#collect-toc(items => toc-list(items))
```

The same data can drive desktop and mobile navigation. Client-side active-link
behavior is an optional `<aster-script>` recipe, not part of heading discovery.

#callout(kind: "caution", title: "Scope show rules")[
  Show rules are lexical. Apply the heading rule in the function that receives
  `entry.render()` content, as the docs template does, so headings inside the
  entry are transformed before the TOC query runs.
]
