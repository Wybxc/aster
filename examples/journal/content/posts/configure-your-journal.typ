#import "/templates/content.typ": post

#show: post.with(
  title: "Configure your Papertrail journal",
  description: "Tailor the journal's identity, post lists, and social links.",
  author: "Papertrail Editors",
  date: "2022-09-23T04:58:53Z",
  modified: "2026-06-03T00:00:00Z",
  tags: ("configuration", "docs"),
  featured: true,
)

Most site-wide choices belong in `aster.toml`. Keeping them together makes the
templates reusable and gives every page the same title, description, author,
and canonical base URL.

= Site identity

```toml
[site]
title = "My Blog"
description = "Notes on design and software."
url = "https://example.com/"
language = "en"
author = "Your name"
```

Use an absolute URL ending in `/`; feeds, canonical links, and the sitemap build
their addresses from it.

= Post lists

The `per-page` value controls paginated post and tag pages. `per-index` limits
the Recent Posts section without changing the complete archive.

```toml
[posts]
per-page = 4
per-index = 4
```

= Social links

Add only the services you actively use. Each item supplies its accessible name
and destination, while the shared social component selects the corresponding
icon.
