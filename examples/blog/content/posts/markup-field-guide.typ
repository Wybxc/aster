#metadata((
  title: "A field guide to Typst markup",
  description: "A sample of Typst markup used when writing Aster content.",
  date: "2024-06-19",
  image: "/assets/cover-markup.jpg",
)) <aster-frontmatter>

Here is a sample of the Typst markup available when writing content in Aster.

= Headings

Typst headings map to semantic HTML heading levels. Additional `=` characters
create deeper sections.

== A second-level heading

=== A third-level heading

= Paragraphs and emphasis

Paragraphs are separated by a blank line. Use *strong emphasis*, _emphasis_,
and `inline code` without leaving markup mode.

= Images

#html.img(src: "/assets/about-studio.jpg", alt: "A bright studio desk beside a city window")

= Blockquotes

#quote[
  Don't communicate by sharing memory, share memory by communicating.
]

= Lists

- Fruit
  - Apple
  - Orange
- Dairy
  - Milk
  - Cheese

+ First item
+ Second item
+ Third item

= Code blocks

```html
<!doctype html>
<html lang="en">
  <head><title>Example document</title></head>
  <body><p>Test</p></body>
</html>
```
