#import "/templates/content.typ": post

#show: post.with(
  title: "Add comments as a progressive enhancement",
  description: "Add GitHub Discussions-backed comments as an optional browser enhancement.",
  author: "Papertrail Editors",
  date: "2024-07-25T11:11:53Z",
  modified: "2025-03-12T12:28:53Z",
  tags: ("aster", "blog", "docs"),
)

Giscus provides comments through GitHub Discussions. The static article remains
complete without it; a browser script loads the discussion interface when the
visitor reaches the comments section.

= Prepare the repository

The repository must be public, Discussions must be enabled, and the Giscus app
must have access. Use the setup form on the Giscus website to select a repository,
discussion category, and page-to-discussion mapping.

= Add the client script

Place the generated script element near the end of the article template or in a
project-specific comments component. Keep repository identifiers in configuration
instead of repeating them in each post.

```html
<script
  src="https://giscus.app/client.js"
  data-mapping="pathname"
  data-theme="preferred_color_scheme"
  crossorigin="anonymous"
  async>
</script>
```

= Follow the active theme

When the reader switches color schemes, send the new theme to the embedded
Giscus frame. Comments should follow the page without controlling the page's
own theme state.
