# Aster comprehensive example

This project demonstrates Aster's current public behavior through three lazy
content collections, one static page, and three dynamic route shapes that also
include nested content ids. The templates use native Typst `#show` rules,
markup, scripting, mathematics, tables, frames, images, and packages, while
Aster handles recursive CSS imports, syntax highlighting, and
content-addressed asset publication without requiring business classes in the
source templates. The `src/rss.xml.typ` endpoint uses the Universe `exemel`
package to publish an RSS feed from the journal collection; RSS policy remains
ordinary project-owned Typst code.

The following command builds the site once:

```sh
cargo run -- build -p examples/site
```

The following command keeps the project running and rebuilds affected pages
when their tracked inputs change:

```sh
cargo run -- watch -p examples/site
```

The generated site is written to `examples/site/dist/`.
