#import "/components/content.typ": callout

#metadata((
  title: "Taxonomies",
  description: "Derive tag, category, and archive indexes from entry metadata.",
  section: "Guides",
  section_order: 20,
  order: 40,
)) <aster-frontmatter>

Taxonomies are dictionaries derived from entry metadata. This example keeps a
display name, a URL slug, and a count for each tag:

```typ
#let tag-slug(tag) = lower(tag).replace(" ", "-")

#let all-tags(posts) = {
  let tags = (:)
  for item in posts {
    for tag in item.metadata.tags {
      tags.insert(tag-slug(tag), tag)
    }
  }
  tags.pairs()
    .map(pair => (
      slug: pair.first(),
      name: pair.last(),
      count: posts.filter(item =>
        item.metadata.tags.any(tag => tag-slug(tag) == pair.first())
      ).len(),
    ))
    .sorted(key: tag => tag.slug)
}
```

The tag page can use the result to declare dynamic route parameters:

```typ
#import "/lib.typ": published-posts, route-params

#metadata(
  all-tags(published-posts()).map(tag => (tag: tag.slug))
) <aster-route>
```

The page then finds the matching entry and filters the same published query:

```typ
#let tag = route-params.at("tag", default: "")
#let info = all-tags(published-posts()).find(item => item.slug == tag)
#let name = if info == none { tag } else { info.name }
#let posts = published-posts().filter(item =>
  item.metadata.tags.any(value => tag-slug(value) == tag)
)
```

= Categories and archives

The same pattern works for any metadata field. For a category, replace the
inner tag loop with one value. For an archive, normalize a date into a year or
month and group entries in a dictionary:

```typ
#let by-year(posts) = {
  let years = (:)
  for item in posts {
    let year = item.metadata.date.slice(0, 4)
    years.insert(year, years.at(year, default: ()) + (item,))
  }
  years.pairs().sorted(key: pair => pair.first()).rev()
}
```

Keep normalization, display names, and ordering in the project library. Aster
only supplies the lazy entries and route planning.

#callout(kind: "note", title: "Stable slugs")[
  If changing a label must not break old links, store the slug in frontmatter
  instead of deriving it from the display name on every build. Redirects for
  renamed slugs are then another project-owned generator recipe.
]
