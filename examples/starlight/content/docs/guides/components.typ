#import "/components/content.typ": aside, card-grid, doc-heading, steps

#metadata((
  title: "Components",
  description: "Compose reusable documentation UI from Typst functions.",
  section: "Guides",
  section_order: 20,
  order: 20,
  badge: "UI",
  toc: (
    (id: "callouts", title: "Callouts", level: 2),
    (id: "cards", title: "Cards", level: 2),
    (id: "steps", title: "Steps", level: 2),
  ),
)) <aster-frontmatter>

Components accept structured values and return content. Their styles and scripts
remain next to the rendering function and are deduplicated per page.

#doc-heading(id: "callouts")[Callouts]

#aside(kind: "note")[Use notes for useful context that is not part of the main instruction.]

#aside(kind: "tip")[Use tips for an optional technique that can improve a workflow.]

#aside(kind: "caution")[Use cautions when a choice may have surprising consequences.]

#aside(kind: "danger")[Use danger notices for actions that can lose data or break a deployment.]

#doc-heading(id: "cards")[Cards]

#card-grid((
  (title: "Content", body: [Write the document with Typst markup and functions.]),
  (title: "Structure", body: [Use semantic HTML for the generated browser document.]),
  (title: "Resources", body: [Attach component CSS and JavaScript through metadata.]),
))

#doc-heading(id: "steps")[Steps]

#steps((
  (title: "Define the API", body: [Choose explicit arguments that encode the component invariant.]),
  (title: "Return content", body: [Build semantic HTML with typed elements where available.]),
  (title: "Attach resources", body: [Keep component-owned behavior in the same Typst source module.]),
))
