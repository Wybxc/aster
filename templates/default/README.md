# Aster site

Run `aster dev` to serve the site locally with automatic browser refresh, or
run `aster build` to generate it once. Files in `public/` are copied unchanged
to the root of the generated `dist/` site; this is useful for host-level files
such as `CNAME` that are not page resources. Route templates live in `pages/`,
while project styles live in `styles/`.

Typst reads project settings directly with `toml("/aster.toml")`. Aster reserves
`sys.inputs._aster` for its runtime protocol. It contains the Aster version,
lazy content collections, and the current route's URL path and parameter
dictionary, together with the complete planned page and endpoint URL lists.
Route data is absent during editor evaluation and route probing.

Project resources referenced by HTML are published with content-addressed file
names. A resource path such as `/assets/logo.svg` resolves from the project root,
not from the website root; protocol URLs and `//` references remain external.
Use `rel="stylesheet"` for ordinary CSS and `rel="tailwind"` for Tailwind input.

Components and pages can declare managed styles, classic scripts, and ES
modules with `metadata(path) <aster-style>`, `metadata(path) <aster-script>`, and
`metadata(path) <aster-module>`. Fenced `css` and `js` raw blocks are accepted in
place of a path and are published as generated files. Classic scripts are loaded
from the document head with `defer`; modules are bundled with the external
`esbuild` executable and loaded with `type="module"`. Local HTML
`<script type="module">` elements are bundled as well: relative `src` values use
the source file that produced the element as their base, while inline module code
is extracted to a file. Templates can instead place ordinary `html.link` and
`html.script` elements directly in the document head.
