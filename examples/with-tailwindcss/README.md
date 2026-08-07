# Aster with Tailwind CSS

This port runs the external Tailwind CSS v4 CLI through `rel="tailwind"`, then
passes the result through Aster's normal Lightning CSS pipeline. Install the
`tailwindcss` executable separately before building.

```sh
cargo run -- dev -p examples/with-tailwindcss
```

The original example design is distributed under `ASTRO-LICENSE`. Its npm-only
confetti dependency is replaced with a small component-owned browser script.
