#import "/components/content.typ": callout

#metadata((
  title: "Heading IDs and Tables of Contents",
  description: "Collect headings from rendered Typst content and render a page TOC.",
  section: "Guides",
  section_order: 20,
  order: 42,
)) <aster-frontmatter>

Heading ids and TOC entries are page-template policy, not content collection
behavior.

= Heading ids

Extract readable text from a heading body, then normalize it into a stable
fragment id. Keep letters, numbers, spaces, underscores, and hyphens, and
collapse runs of separators into single hyphens:

```typ
#let heading-text(content) = {
  if type(content) == str {
    content
  } else if repr(content) == "[ ]" {
    " "
  } else {
    let fields = content.fields()
    let text = fields.at("text", default: none)
    if text != none {
      text
    } else {
      let children = fields.at("children", default: none)
      if children != none {
        children.map(heading-text).join("")
      } else {
        let body = fields.at("body", default: none)
        if body != none { heading-text(body) } else { "" }
      }
    }
  }
}

#let heading-id(title) = {
  lower(title)
    .replace(regex("[^\\p{L}\\p{N} _-]"), "")
    .replace(regex("[ _]+"), "-")
    .replace(regex("-+"), "-")
    .trim(regex("[- ]"))
}
```

Repeated headings produce the same id; add a project-specific uniquifier when
anchors must be unique. CJK and other Unicode letters are preserved by the
character class above, so anchors stay readable.

= Apply a show rule

Render every native heading with an id and a private label the TOC query can
collect. Wrap `entry.render()` so the rule applies to the entry's headings:

```typ
#let heading-element(body, id, level) = [
  #html.elem("h" + str(level), attrs: (id: id))[#body] <aster-doc-toc-heading>
]

#let with-heading-rules(body) = {
  show heading: it => {
    let id = heading-id(heading-text(it.body))
    heading-element(it.body, id, it.level + 1)
  }
  body
}
```

Use a namespaced label so content authors cannot accidentally collide with the
recipe's query. The docs example also attaches an anchor link and hover styling
inside `heading-element`; that presentation is not part of heading discovery.

= Collect heading metadata

A `context` function can query every label after the body has been evaluated
and pass `(id, title, level)` records to the TOC renderer:

```typ
#let collect-toc(use-toc) = context {
  use-toc(query(<aster-doc-toc-heading>).map(heading => (
    id: heading.attrs.at("id", default: ""),
    title: heading-text(heading.body),
    level: int(heading.tag.slice(1)),
  )))
}
```

The query observes the complete document regardless of where the heading was
created.

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
