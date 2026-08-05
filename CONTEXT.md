# Aster domain context

Aster builds Typst-authored sites into a complete static output tree.

## Project

An **Aster project** is a directory containing `aster.toml`. Its conventional directories are:

- `pages/` — page and generated-endpoint route templates
- `content/` — content entries grouped into collections
- `styles/` — project CSS sources processed through Aster
- `assets/` — project resources read by Typst or referenced from CSS
- `public/` — files copied unchanged to the output root
- `dist/` — the published output tree

Project discovery selects the nearest ancestor containing an `aster.toml` file. A build requires `pages/`; `content/`, `styles/`, `assets/`, and `public/` are optional. The project owns watch-path policy: configuration, structural directories, tracked build dependencies, and project-relative `[watch].paths` are watched while `dist/` is always excluded. An additional watch path is classified as a file or recursive directory from its current filesystem state; missing paths remain dependencies and are reclassified after creation. The project root and paths overlapping the output directory are rejected.

Like Typst's standard filesystem loader, project paths retain their absolute lexical form instead of being canonicalized. Operating-system absolute paths and `..` paths that escape the project root are rejected, while filesystem access follows symbolic links even when their targets are outside the project root. A leading `/` in a project-source interface denotes the project virtual root; paths within that namespace are computed through Typst's `VirtualPath` model.

## Content protocol

The **content protocol** is the `_aster` Typst input. Rust owns its version and complete value, including the empty state. It maps each collection and entry id to a lazy entry module. Each module exposes `id`, `collection`, and a Typst `render` closure; it does not expose a source path or contain evaluated content or frontmatter.

`_aster` is reserved. Route parameters also cannot replace configuration inputs.

The non-rendering content helpers in `templates/default/lib.typ` expose the protocol through `get-collection`, `get-collection-ids`, and `get-entry`. The file is imported directly by consumers and does not aggregate components or templates. The helpers return the Rust-provided entry modules unchanged. Calling `entry.metadata()` runs a Rust-constructed Typst user closure that dynamically imports the entry source and extracts `<aster-frontmatter>` metadata; `entry.render()` imports the same source and returns its content. Typst's memoized module evaluation is shared when both are called in one build context. These imports become tracked `World::source` dependencies, so editing an entry invalidates only pages that accessed it. Route declarations use `get-collection-ids` when they only need membership and should not depend on entry bodies. Adding, removing, or renaming an entry changes the shared entry manifest and invalidates all page libraries.

## Route plan

A `.typ` file under `pages/` is evaluated while planning the output tree. A template containing one `<aster-endpoint>` metadata declaration is a generated endpoint; every other source is a page template. A static endpoint declaration must contain its final string or bytes. A dynamic endpoint uses the same `<aster-route>` metadata as a dynamic page: the first evaluation discovers its parameter sets, then Aster evaluates the template again with each parameter set and extracts that route's string or bytes from `<aster-endpoint>`. The probe declaration may contain `none` because its value is discarded. An endpoint's exact output path substitutes its route parameters and removes only the final `.typ` extension. Page output paths instead append `.html`, so directory URLs are expressed explicitly with templates such as `posts/[...slug]/index.typ`. A page template without bracket parameters defines one static route. A page template with `[name]` or `[...name]` parameters is a **dynamic route** and declares parameter sets through `<aster-route>` metadata.

A **route plan** is the deterministic, collision-free set of pages and generated endpoints produced before rendering. It owns template discovery, evaluation and classification, route and endpoint metadata validation, parameter matching, output confinement, warnings, ordering, and collisions.

A normal parameter fills one path segment and cannot contain separators. A spread parameter is a standalone segment and may expand into multiple validated segments. Generated output paths are always relative to `dist/` and cannot contain `.` or `..` components.

## Build session

A **build session** is the reusable public build interface bound to one Aster project. It loads the project's configuration for each build and owns an internal Typst build session with shared fonts, package and project-file access, tracked source and content discovery, input libraries, Typst world construction, page compilation, and source-aware diagnostics. The project-file store is the tracked filesystem surface for directory membership, dynamically imported content, and build transforms that need incremental file access.

The session is reused across builds. The first build compiles directly. Before each later build, it marks loaded files stale; subsequent reads update Typst sources in place so comemo can validate and reuse unchanged compilation and transformation results. After the build attempt, it ages the global comemo cache. Page compilation is memoized through a tracked Typst world.

Callers do not construct or track Typst worlds. Source and content listings are memoized through the session's tracked project-file surface, so directory membership changes invalidate discovery. CSS bundling uses the same surface: path resolution and every entry or transitive import read become comemo constraints. A `rel=\"css\"` link selects standard CSS bundling, while `rel=\"tailwind\"` runs that entry through the one-shot external Tailwind CLI before the same Lightning CSS post-processing. Aster conservatively records each Tailwind entry tree and conventional Tailwind configuration files because arbitrary filesystem reads by the child process cannot enter the tracked dependency graph. Page compilation remains memoized through the tracked Typst world.

## Document transform

A **document transform** is the single ordered traversal from a compiled Typst HTML document to a publishable page. It owns CSS-link bundling, large data-image extraction, syntax highlighting, and highlight-stylesheet injection. The transform visits each element once; CSS, image, and highlight implementations remain internal rather than exposing independent passes.

## Output publication

An **output publication** is the complete candidate output tree for one successful build. It owns:

- output-path confinement
- lexical source-reference resolution relative to the actual page template or project virtual root
- generated-asset identity and content-addressed naming
- browser-facing references relative to each output page
- deduplication
- deterministic replacement of `dist/`
- removal of stale pages and assets

Rendering, endpoint evaluation, and document transformation accumulate a publication in memory. Once every route succeeds, publication clears `dist/` and writes the complete output tree directly.

## Build outcome

A **build outcome** records published pages and generated endpoints separately, together with collected warnings and elapsed time. Internal assets and copied public files are not reported as authored routes. Build modules decide whether an operation succeeds and preserve diagnostic context. Successful init and build commands return outcomes; the terminal adapter renders them and the CLI maps command results to process exit status in one place.

Aster warnings are non-fatal by explicit policy. Page compilation, route planning, transformation, and output publication failures are fatal. Failures before publication leave the prior `dist/` untouched; an output write failure may leave a partial output tree.
