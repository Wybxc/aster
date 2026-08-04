#import "/templates/content.typ": post

#show: post.with(
  title: "How to update dependencies of AstroPaper",
  description: "A cautious workflow for updating the tools used to build the theme.",
  author: "Sat Naing",
  date: "2023-07-20T15:33:05Z",
  tags: ("FAQ",),
)

Tool updates are easiest to review when they are small and reproducible. Record
the versions used in development and CI, update one boundary at a time, and
compare the generated site before accepting the change.

= Inspect the current toolchain

```sh
aster --version
tailwindcss --help
typst --version
```

This port uses the standalone Tailwind executable, so it does not require a
JavaScript package manifest. A build still verifies that Tailwind output can be
parsed and optimized before publication.

= Rebuild every route

After updating a compiler, run a complete build and the repository test suite.
Review page count, generated endpoints, CSS output, and representative light,
dark, mobile, and desktop pages.

= Keep rollback straightforward

Commit dependency changes separately from content edits. If an output regression
appears, the smaller diff makes its source easier to isolate.
