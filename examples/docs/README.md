# Aster documentation example

This example implements a complete documentation site focused on navigation and
reading. Its content documents Aster's current project model, CLI, content
protocol, routing, resource processing, configuration, generators,
postprocessing, and Rust library API. The site itself includes hierarchical
navigation, automatic heading IDs, a table of contents, previous/next links,
edit links, code highlighting, theme selection, and a responsive mobile sidebar.

```sh
cargo run -- dev -p examples/docs
```

Build static output with:

```sh
cargo run -- build -p examples/docs
```

The implementation is intentionally self-contained: content is stored in a
collection, routes are generated from entry IDs, and the search box filters the
loaded navigation without an external indexer.

The current branding and visual assets are original to Aster. A design notice
from an earlier revision is preserved in `UPSTREAM-LICENSE`.
