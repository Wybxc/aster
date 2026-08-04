# AsterPaper

This example ports the information architecture and visual character of
[AstroPaper](https://github.com/satnaing/astro-paper) to Aster. It is a real
Typst-authored site rather than a direct source translation: content metadata,
pagination, tag routes, archives, adjacent-post navigation, and generated files
are all implemented with Aster's native APIs.

The example includes responsive navigation, light and dark themes, Tailwind
Typography, article heading links, copy buttons, a reading progress indicator,
RSS, `robots.txt`, and a sitemap. Pagefind and generated Open Graph images are
intentionally omitted.

Install the standalone Tailwind CSS CLI as `tailwindcss`, then run:

```sh
cargo run -- build -p examples/astro-paper
cargo run -- dev -p examples/astro-paper
```

AstroPaper is Copyright (c) 2023 Sat Naing and distributed under the MIT
license; the complete notice is preserved in `ASTROPAPER-LICENSE`. This example
reuses the upstream favicon and refers to AstroPaper's public design, article
topics, and metadata. Its Typst implementation and article prose were written
for this repository; upstream article bodies and code samples were not copied.
