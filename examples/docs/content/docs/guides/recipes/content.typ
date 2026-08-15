#import "/components/content.typ": callout

#metadata((
  title: "Content Queries",
  description: "Build project-owned collection queries without adding a Rust-side content type.",
  section: "Guides",
  section_order: 20,
  order: 37,
)) <aster-frontmatter>

The #link("/guides/content-collections/")[Content Collections] guide introduces
the lazy `get-collection`, `get-entry`, and `get-collection-ids` helpers. Wrap
them into domain queries instead of adding a Rust-side post type. Metadata is
project-defined, so a publication can build its published view entirely in
Typst:

```typ
#import "/lib.typ": get-collection

#let published-posts() = {
  let today = datetime.today().display(
    "[year]-[month padding:zero]-[day padding:zero]",
  )
  get-collection("posts")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .filter(item => not item.metadata.at("draft", default: false))
    .filter(item => item.metadata.date.slice(0, 10) <= today)
    .sorted(key: item => item.metadata.date)
    .rev()
}
```

Keep this function in the project's `lib.typ`. Other projects may use a
different draft policy, date format, or ordering key. One shared ordering can
then drive adjacent links, archives, and feeds; the
#link("/guides/recipes/navigation/")[Adjacent Navigation] recipe turns that
ordering into previous and next links.

#callout(kind: "note", title: "Protocol scope")[
  `_aster.collections` transports entries and their lazy accessors. It does
  not decide what a post, draft, tag, archive, or translation is. Those rules
  belong in the project's Typst library.
]
