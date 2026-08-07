# Aster documentation example

This example implements a complete documentation site with Aster and Typst. It
includes hierarchical navigation, automatic heading IDs, a table of contents,
previous/next links, edit links, code highlighting, theme selection, and a
responsive mobile sidebar.

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
