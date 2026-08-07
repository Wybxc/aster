#metadata((
  title: "Notes on quiet maintenance",
  description: "How routine cleanup keeps a project understandable.",
  date: "2022-07-22",
  image: "/assets/cover-third.jpg",
)) <aster-frontmatter>

Maintenance is most effective before it becomes a dedicated project. Removing
an obsolete option, tightening a module boundary, or updating an example while
the context is fresh keeps the codebase easier to navigate.

= Leave a clear trail

Small changes should leave evidence: focused tests, current examples, and names
that still describe the behavior. That trail reduces the cost of the next
change more reliably than a large cleanup scheduled for later.
