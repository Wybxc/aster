#metadata((
  title: "Small tools, clear boundaries",
  description: "Why focused utilities often outlast ambitious internal platforms.",
  date: "2022-07-15",
  image: "/assets/cover-second.jpg",
)) <aster-frontmatter>

A useful internal tool begins with a narrow promise. It accepts a small set of
inputs, produces an observable result, and leaves policy decisions to the layer
that owns them.

= Make ownership visible

Clear names and explicit data flow make a utility easier to replace. A helper
should reduce local complexity without becoming a second hidden application.

When the boundary stays small, tests become direct and failures point to the
actual operation rather than a long chain of adapters.
