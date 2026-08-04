# Aster site

Run `aster dev` to serve the site locally with automatic browser refresh, or
run `aster build` to generate it once. Files in `public/` are copied unchanged
to the root of the generated `dist/` site. Route templates live in `pages/`,
while project styles live in `styles/`.

Components and templates can declare managed styles and classic scripts with
`metadata(path) <style>` and `metadata(path) <script>`. Fenced `css` and `js`
raw blocks are accepted in place of a path and are published as generated files.
Managed scripts are loaded from the document head with `defer`.
