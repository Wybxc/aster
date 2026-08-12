# Aster examples

Each example has a distinct primary purpose, while complete sites still combine
the supporting features needed to be realistic. Start with a focused example
when learning one mechanism, choose a complete site by product shape, and use
the showcase when surveying Aster's API as a whole.

## Focused examples

These projects minimize unrelated structure without omitting anything required
for the demonstrated workflow.

| Example | Focus |
| --- | --- |
| [`minimal`](minimal/) | The smallest valid Aster project: one page and one asset. |
| [`basics`](basics/) | Project structure, layouts, components, component-owned CSS, and published assets. |
| [`with-tailwindcss`](with-tailwindcss/) | The external Tailwind CSS CLI integration. |

## Complete sites

These are usable site implementations. Their supporting features overlap where
the site type calls for it; their primary product and content models differ.

| Example | Primary site model |
| --- | --- |
| [`blog`](blog/) | A compact editorial blog with posts, dynamic routes, Atom, and a sitemap. |
| [`docs`](docs/) | A documentation site with hierarchical navigation, heading IDs, a table of contents, and client-side navigation controls. |
| [`journal`](journal/) | A publication with content collections, pagination, tags, archives, Atom, sitemap, and robots generators. |
| [`portfolio`](portfolio/) | Nested content IDs, dynamic project routes, image-heavy presentation, and a small theme interaction. |
| [`release-log`](release-log/) | A changelog that renders releases both as a chronological stream and as detail pages. |

The blog is deliberately smaller than the journal: it is the approachable
editorial baseline, while the journal demonstrates richer publication
navigation and metadata workflows.

## Comprehensive reference

[`site`](site/) combines multiple content collections, static and dynamic route
shapes, generators, packages, syntax highlighting, CSS imports, images, frames,
mathematics, and Typst scripting. It is intentionally broad and serves as a
reference and compatibility fixture rather than as a starter template.

General framework behavior belongs in tests. The default project created by
`aster init` lives in [`../templates/default`](../templates/default/) and is
maintained separately from these demonstrations.

Image licensing and provenance are recorded in [`IMAGE-CREDITS.md`](IMAGE-CREDITS.md).
