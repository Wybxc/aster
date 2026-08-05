# Aster site

Run `aster dev` to serve the site locally with automatic browser refresh, or
run `aster build` to generate it once. Files in `public/` are copied unchanged
to the root of the generated `dist/` site. Route templates live in `pages/`,
while project styles live in `styles/`.

Components and templates can declare managed styles, classic scripts, and ES
modules with `metadata(path) <aster-style>`, `metadata(path) <aster-script>`, and
`metadata(path) <aster-module>`. Fenced `css` and `js` raw blocks are accepted in
place of a path and are published as generated files. Classic scripts are loaded
from the document head with `defer`; modules are bundled with the external
`esbuild` executable and loaded with `type="module"`. Local HTML
`<script type="module">` elements are bundled as well: relative `src` values use
the page template as their base, while inline module code is extracted to a file.
