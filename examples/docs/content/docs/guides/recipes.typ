#import "/components/content.typ": callout

#metadata((
  title: "Recipes",
  description: "Build site-specific features from Aster's small Typst-native primitives.",
  section: "Guides",
  section_order: 20,
  order: 35,
)) <aster-frontmatter>

Aster does not define a blog, documentation, or publication schema. A recipe is
ordinary Typst code that combines collections, route metadata, the rendered
site snapshot, and exact-path generators.

The recipes in this section are extracted from the complete examples. They are
project-owned patterns, not additional Aster runtime features:

- #link("/guides/recipes/settings/")[Project settings and URLs]
- #link("/guides/recipes/content/")[Content queries]
- #link("/guides/recipes/routes/")[Dynamic routes]
- #link("/guides/recipes/pagination/")[Pagination]
- #link("/guides/recipes/taxonomy/")[Taxonomies]
- #link("/guides/recipes/navigation/")[Adjacent navigation]
- #link("/guides/recipes/toc/")[Heading IDs and tables of contents]
- #link("/guides/recipes/feeds/")[Atom, sitemap, and robots generators]

#callout(kind: "tip", title: "Source examples")[
  `examples/journal/lib.typ` contains the content queries used by a
  publication. Its page and generator templates combine those queries into
  pagination, tags, archives, Atom, sitemap, and robots output. The docs
  template uses the heading and TOC recipe for its own content collection.
]

#callout(kind: "note", title: "Choose the narrowest phase")[
  Use a page when the result is part of the navigable site. Use a generator when
  it is an exact-path file derived from routes or final page HTML. Use a
  postprocessor when an external program needs the complete staged filesystem.
]
