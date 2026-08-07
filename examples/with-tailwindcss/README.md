# Aster with Tailwind CSS

This port runs the external Tailwind CSS v4 CLI through `rel="tailwind"`, then
passes the result through Aster's normal Lightning CSS pipeline. Install the
`tailwindcss` executable separately before building.

```sh
cargo run -- dev -p examples/with-tailwindcss
```

The example uses original Aster branding and a small component-owned browser
script for the confetti interaction. An upstream notice from an earlier version
is preserved in `UPSTREAM-LICENSE`.
