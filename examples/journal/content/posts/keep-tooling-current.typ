#import "/templates/content.typ": post

#show: post.with(
  title: "Keep the journal toolchain current",
  description: "A cautious workflow for updating the tools used to build the theme.",
  author: "Papertrail Editors",
  date: "2023-07-20T15:33:05Z",
  tags: ("FAQ",),
)

Tool updates are easiest to review when they are small and reproducible. Record
the versions used in development and CI, update one boundary at a time, and
compare the generated site before accepting the change.

= Inspect the current toolchain

```sh
aster --version
esbuild --version
typst --version
```

This port uses Aster's built-in CSS pipeline and the standalone esbuild
executable for JavaScript modules, so it does not require a JavaScript package
manifest. A build still verifies all transformed assets before publication.

= Rebuild every route

After updating a compiler, run a complete build and the repository test suite.
Review page count, generated endpoints, CSS output, and representative light,
dark, mobile, and desktop pages.

= Keep rollback straightforward

Commit dependency changes separately from content edits. If an output regression
appears, the smaller diff makes its source easier to isolate.
