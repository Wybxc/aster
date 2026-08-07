#import "/templates/content.typ": post

#show: post.with(
  title: "Writing a new Papertrail post",
  description: "Practical conventions for creating and organizing journal entries.",
  author: "Papertrail Editors",
  date: "2022-09-23T15:22:00Z",
  modified: "2026-06-03T00:00:00Z",
  tags: ("docs",),
  featured: true,
)

A Papertrail post combines a small metadata record with a document body. Keep
the record predictable and let the file location provide the stable post id.

= Create an entry

Add a source file under `content/posts/`. Nested folders are useful when a
larger site needs to organize articles by subject or year; the nested id remains
part of the generated route.

```typ
#show: post.with(
  title: "My first post",
  description: "A short summary for lists and feeds.",
  author: "Your name",
  date: "2026-08-04",
  tags: ("notes",),
)

Write the article here.
```

= Choose useful metadata

Titles and descriptions appear in post lists and document metadata. Dates
control sorting, while tags create topic pages. Mark a post as featured when it
should appear in the dedicated section on the home page.

= Keep drafts out of public routes

Set `draft: true` while an entry is unfinished. The shared collection helpers
exclude drafts from article routes, archives, feeds, tags, and the sitemap.
