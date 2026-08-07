#let post(
  title: "Untitled",
  description: "",
  author: "",
  date: "1970-01-01",
  modified: none,
  canonical: none,
  tags: ("others",),
  featured: false,
  draft: false,
  body,
) = [
  #metadata((
    title: title,
    description: description,
    author: author,
    date: date,
    modified: modified,
    canonical: canonical,
    tags: tags,
    featured: featured,
    draft: draft,
  )) <aster-frontmatter>
  #body
]

#let page(title: "Untitled", description: "", body) = [
  #metadata((title: title, description: description)) <aster-frontmatter>
  #body
]
