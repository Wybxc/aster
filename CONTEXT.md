# Aster domain context

Aster builds Typst-authored sites into a complete static output tree.

## Project

An **Aster project** is a directory containing `aster.toml`. Its conventional directories are:

- `src/` — page templates
- `content/` — content entries grouped into collections
- `dist/` — the published output tree

Project discovery selects the nearest ancestor containing an `aster.toml` file. A build requires `src/`; `content/` is optional.

## Content protocol

The **content protocol** is the `_aster` Typst input. Rust owns its version and complete value, including the empty state. It contains content collections and entries. Each **content entry** has an id, collection name, project-relative content path, compiled Typst body, and frontmatter metadata.

`_aster` is reserved. Route parameters also cannot replace configuration inputs.

The Typst adapter in `lib/aster/content.typ` exposes the protocol through `get-collection`, `get-entry`, and `render`.

## Route plan

A **page template** is a `.typ` file under `src/`. A template without bracket parameters defines one static route. A template with `[name]` or `[...name]` parameters is a **dynamic route** and declares parameter sets through `<route>` metadata.

A **route plan** is the deterministic, collision-free set of pages produced before rendering. It owns template parsing, route metadata validation, parameter matching, output confinement, ordering, and collisions.

A normal parameter fills one path segment and cannot contain separators. A spread parameter is a standalone segment and may expand into multiple validated segments. Generated output paths are always relative to `dist/` and cannot contain `.` or `..` components.

## Typst build session

A **Typst build session** is bound to one Aster project. It owns shared fonts, package and project-file access, input libraries, Typst world construction, content evaluation, page compilation, and source-aware diagnostics. The project-file store is also the tracked filesystem surface for build transforms that need incremental file access.

The session is reused across builds. The first build compiles directly. Before each later build, the build driver marks loaded files stale; subsequent reads update Typst sources in place so comemo can validate and reuse unchanged compilation and transformation results. After the build attempt, the driver ages the global comemo cache. Page compilation is memoized through a tracked Typst world.

Callers do not construct or track Typst worlds. CSS bundling is memoized through the session's tracked project-file surface: path resolution and every entry or transitive import read become comemo constraints. Other document transforms that read files directly remain uncached unless they adopt the same tracked surface or include file content in their cache key.

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

A **build outcome** records published pages, collected warnings, and elapsed time. Build modules decide whether an operation succeeds and preserve diagnostic context. The terminal adapter in `diag.rs` decides only how outcomes are displayed.

Aster warnings are non-fatal by explicit policy. Page compilation, route planning, transformation, and output publication failures are fatal. Failures before publication leave the prior `dist/` untouched; an output write failure may leave a partial output tree.
