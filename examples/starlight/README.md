# Starlight-style documentation example

This example ports Starlight's core documentation experience to Aster without
depending on Astro, MDX, or a JavaScript framework. It includes:

- a fixed header, responsive sidebar, mobile table of contents, and three-column
  desktop layout;
- light, dark, and automatic themes stored in the browser;
- sidebar navigation and previous/next links generated from content metadata;
- title and description search across the documentation collection;
- anchored headings with scroll tracking, callouts, cards, steps, tabs, syntax
  highlighting, and code-copy controls;
- edit links, a static 404 page, and paths that remain valid when the output is
  deployed below a subdirectory.

Run the development server from the Aster repository root:

```sh
cargo run -- dev -p examples/starlight
```

Build the static output once with:

```sh
cargo run -- build -p examples/starlight
```

Documentation entries live under `content/docs/`. Their `<aster-frontmatter>`
metadata controls the page title, description, sidebar group and order, badge,
and table of contents. `pages/[...slug]/index.typ` probes the collection and
renders every non-root entry through one route template.

The built-in search filters page titles and descriptions in the browser. It is
deliberately smaller than Starlight's Pagefind-powered full-text search and does
not claim Pagefind compatibility.

Layout values and visual behavior adapted from Astro Starlight are used under
its MIT license in `STARLIGHT-LICENSE`. The Typst implementation and example
content are part of Aster.
