# Aster domain context

Aster builds Typst-authored sites into a complete static output tree.

## Project

An **Aster project** is a directory containing `aster.toml`. Its conventional directories are:

- `src/` — page templates
- `content/` — content entries grouped into collections
- `dist/` — the published output tree

Project discovery selects the nearest ancestor containing an `aster.toml` file. A build requires `src/`; `content/` is optional. The project owns watch-path policy: configuration, structural directories, and tracked build dependencies are watched while `dist/` is always excluded.

## Content protocol

The **content protocol** is the `_aster` Typst input. Rust owns its version and complete value, including the empty state. It maps each collection and entry id to a lazy entry module. Each module exposes `id`, `collection`, and a Typst `render` closure; it does not expose a source path or contain evaluated content or frontmatter.

`_aster` is reserved. Route parameters also cannot replace configuration inputs.

The Typst adapter in `templates/default/lib/aster/content.typ` exposes the protocol through `get-collection`, `get-collection-ids`, and `get-entry`, returning the Rust-provided entry modules unchanged. Calling `entry.render()` runs a Rust-constructed Typst user closure that dynamically imports the entry source, includes its content, extracts labelled frontmatter, and returns `(metadata: ..., content: ...)`. These imports become tracked `World::source` dependencies, so editing an entry invalidates only pages that rendered it. Route declarations use `get-collection-ids` when they only need membership and should not depend on entry bodies. Adding, removing, or renaming an entry changes the shared entry manifest and invalidates all page libraries.

## Route plan

A **page template** is a `.typ` file under `src/`. A template without bracket parameters defines one static route. A template with `[name]` or `[...name]` parameters is a **dynamic route** and declares parameter sets through `<route>` metadata.

A **route plan** is the deterministic, collision-free set of pages produced before rendering. It owns template discovery, parsing, dynamic-template probing through the Typst build session, route metadata validation, parameter matching, output confinement, warnings, ordering, and collisions.

A normal parameter fills one path segment and cannot contain separators. A spread parameter is a standalone segment and may expand into multiple validated segments. Generated output paths are always relative to `dist/` and cannot contain `.` or `..` components.

## Typst build session

A **Typst build session** is bound to one Aster project. It owns shared fonts, package and project-file access, tracked source and content discovery, input libraries, Typst world construction, page compilation, and source-aware diagnostics. The project-file store is the tracked filesystem surface for directory membership, dynamically imported content, and build transforms that need incremental file access.

The session is reused across builds. The first build compiles directly. Before each later build, the build driver marks loaded files stale; subsequent reads update Typst sources in place so comemo can validate and reuse unchanged compilation and transformation results. After the build attempt, the driver ages the global comemo cache. Page compilation is memoized through a tracked Typst world.

Callers do not construct or track Typst worlds. Source and content listings are memoized through the session's tracked project-file surface, so directory membership changes invalidate discovery. CSS bundling uses the same surface: path resolution and every entry or transitive import read become comemo constraints. Page compilation emits its source template only from inside the memoized body, so cache hits remain quiet.

## Document transform

A **document transform** is the single ordered traversal from a compiled Typst HTML document to a publishable page. It owns CSS-link bundling, large data-image extraction, syntax highlighting, and highlight-stylesheet injection. The transform visits each element once; CSS, image, and highlight implementations remain internal rather than exposing independent passes.

## Output publication

An **output publication** is the complete candidate output tree for one successful build. It owns:

- output-path confinement
- source-reference resolution relative to the actual page template
- generated-asset identity and content-addressed naming
- browser-facing references relative to each output page
- deduplication
- deterministic replacement of `dist/`
- removal of stale pages and assets

Rendering and document transformation accumulate a publication in memory. Once every page succeeds, publication clears `dist/` and writes the complete output tree directly.

## Build outcome

A **build outcome** records published pages, collected warnings, and elapsed time. Build modules decide whether an operation succeeds and preserve diagnostic context. Successful init and build commands return outcomes; the terminal adapter renders them and the CLI maps command results to process exit status in one place.

Aster warnings are non-fatal by explicit policy. Page compilation, route planning, transformation, and output publication failures are fatal. Failures before publication leave the prior `dist/` untouched; an output write failure may leave a partial output tree.
