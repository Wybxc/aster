# Aster domain context

Aster builds Typst-authored sites into a complete static output tree.

## Project

An **Aster project** is a directory containing `aster.toml`. Its conventional directories are:

- `pages/` — HTML page route templates
- `generate/` — Typst generators that produce exact-path files after pages render
- `content/` — content entries grouped into collections
- `styles/` — project CSS sources processed through Aster
- `assets/` — project resources read by Typst or referenced from CSS
- `public/` — files copied unchanged to the output root
- `dist/` — the published output tree

Project discovery selects the nearest ancestor containing an `aster.toml` file. A build requires `pages/`; `generate/`, `content/`, `styles/`, `assets/`, and `public/` are optional. The project owns watch-path policy: configuration, structural directories, tracked build dependencies, project-relative `[watch].paths`, and each postprocessor's `watch` paths are watched while `dist/` is always excluded. An additional watch path is classified as a file or recursive directory from its current filesystem state; missing paths remain dependencies and are reclassified after creation. The project root and paths overlapping the output directory are rejected.

Aster deserializes only its build-owned settings from `aster.toml`. The manifest is not injected into Typst inputs; project Typst code reads the complete value explicitly with `toml("/aster.toml")`. Typst inputs are reserved for Aster's runtime protocol.

Like Typst's standard filesystem loader, project paths retain their absolute lexical form instead of being canonicalized. Operating-system absolute paths and `..` paths that escape the project root are rejected, while filesystem access follows symbolic links even when their targets are outside the project root. A leading `/` in a project-source interface denotes the project virtual root; paths within that namespace are computed through Typst's `VirtualPath` model.

## Content protocol

The **content protocol** is the `_aster` Typst input. Rust owns its version and complete value, including the empty state. It maps each collection and entry id to a lazy entry module. Each module exposes `id`, `collection`, and Typst `metadata` and `render` closures; it does not expose a source path or contain evaluated content or frontmatter.

`_aster.route` is a stable module with native `path(default: none)` and `param(name, default: none)` functions. A concrete compilation supplies their string values through reserved virtual files in its tracked Typst world; a dynamic route probe supplies no route, so the functions return their defaults. `_aster.routes` is another stable module whose native `pages()` function reads the planned page URL set from the same virtual namespace; it returns an empty array before page planning. Route values and the route manifest therefore do not change `sys.inputs` or the `Library`: dynamic probes and every page share the same library, while comemo observes only the runtime values actually read by a compilation. Project helpers keep runtime access lazy and guard the entirely absent `_aster` input for editor evaluation.

`_aster` is reserved. Route parameters cannot replace another existing Typst input.

The non-rendering content helpers in `templates/default/lib.typ` expose the protocol through `get-collection`, `get-collection-ids`, and `get-entry`. The file is imported directly by consumers and does not aggregate components or templates. The helpers return the Rust-provided entry modules unchanged. Calling `entry.metadata()` runs a Rust-constructed Typst user closure that dynamically imports the entry source and extracts `<aster-frontmatter>` metadata; `entry.render()` imports the same source and returns its content. Typst's memoized module evaluation is shared when both are called in one build context. These imports become tracked `World::source` dependencies, so editing an entry invalidates only pages that accessed it. Route declarations use `get-collection-ids` when they only need membership and should not depend on entry bodies. Adding, removing, or renaming an entry changes the shared entry manifest and invalidates all page libraries.

## Route plan

A `.typ` file under `pages/` is always an HTML page template. Page output paths append `.html`, so directory URLs are expressed explicitly with templates such as `posts/[...slug]/index.typ`. A page template without bracket parameters defines one static route. A page template with `[name]` or `[...name]` parameters is a **dynamic route** and declares parameter sets through `<aster-route>` metadata.

A `.typ` file under `generate/` is a **generator**. It runs after all pages have been transformed and encoded, and declares one string or bytes payload with `<aster-output>`. Its exact output path removes only the final `.typ` extension. Dynamic generators use the same `<aster-route>` metadata as dynamic pages. Generator inputs include `_aster.site.pages`; each page has `path`, final `html`, and optional `content` containing final HTML and plain text from the page's unique `<aster-content>` element. `_aster.routes.pages()` returns the complete planned page URL set; generated files are deliberately absent because generators do not form a second navigable route graph.

A **route plan** is a deterministic, collision-free set of outputs. Pages are planned before rendering; generators are planned afterward so their dynamic parameters can use the rendered site snapshot. Planning owns template discovery, parameter matching, output confinement, warnings, ordering, and collisions across both sets.

A normal parameter fills one path segment and cannot contain separators. A spread parameter is a standalone segment and may expand into multiple validated segments. Generated output paths are always relative to `dist/` and cannot contain `.` or `..` components.

## Build session

A **build session** is the reusable public build interface bound to one Aster project. It loads the project's configuration for each build and owns an internal Typst build session with shared fonts, package and project-file access, tracked source and content discovery, phase-specific input libraries, per-compilation route worlds, page compilation, and source-aware diagnostics. The project-file store is the tracked filesystem surface for directory membership, dynamically imported content, and build transforms that need incremental file access.

The session is reused across builds. The first build compiles directly. Before each later build, it marks loaded files stale; subsequent reads update Typst sources in place so comemo can validate and reuse unchanged compilation and transformation results. After the build attempt, it ages the global comemo cache. Page compilation is memoized through a tracked Typst world.

Callers do not construct or track Typst worlds. Source and content listings are memoized through the session's tracked project-file surface, so directory membership changes invalidate discovery. CSS bundling uses the same surface: path resolution and every entry or transitive import read become comemo constraints. A standard `rel=\"stylesheet\"` link selects CSS bundling, while `rel=\"tailwind\"` runs that entry through the one-shot external Tailwind CLI before the same Lightning CSS post-processing. Page compilation remains memoized through the tracked Typst world.

## Document transform

A **document transform** is the single ordered traversal from a compiled Typst HTML document to a publishable page. It owns CSS and script bundling, project-resource publication, large data-image extraction, syntax highlighting, and highlight-stylesheet injection. The transform visits each element once; concrete processors remain internal rather than exposing independent passes.

## Output publication

An **output publication** is the complete candidate output tree for one successful build. It owns:

- output-path confinement
- lexical source-reference resolution relative to the actual page template or project virtual root
- generated-asset identity and content-addressed naming
- browser-facing references relative to each output page
- deduplication
- deterministic replacement of `dist/`
- removal of stale pages and assets

Rendering, generator evaluation, and document transformation accumulate a publication in memory. Once they succeed, Aster writes a staging tree beside the configured output directory. Explicit `[[postprocess]]` commands receive `{site}` for that mutable staging tree. A command may additionally use a private `{output}` directory whose files are imported under its `mount`; `{output}` and `mount` are configured together. Commands are executed directly, without an implicit shell. Only after every postprocessor succeeds does Aster replace the prior output tree.

## Build outcome

A **build outcome** records published pages and generator outputs separately, together with collected warnings and total elapsed time. Internal assets, postprocessor files, and copied public files are not reported as authored outputs. Tracing targets follow their Rust module paths: stages and nested operations under `aster::build` emit spans, while discovered inputs and publication counts emit structured events; CLI lifecycle, status, warning, and error events remain under `aster::cli`. The terminal subscriber preserves those structured records but projects them as readable sentences: the level stays in a fixed column, the sentence follows its span scope indentation, and each completed span appears once with one human-readable duration. Its default level shows stages and authored routes, `-v` includes build-operation details, and `-vv` includes ordinary resource processing; library callers choose their own subscriber and level. Build modules decide whether an operation succeeds and preserve diagnostic context. Successful init and build commands return outcomes; the CLI reports them as structured events and maps command results to process exit status in one place.

Aster warnings are non-fatal by explicit policy. Page compilation, route planning, transformation, generation, postprocessing, and staging failures are fatal and leave the prior `dist/` untouched. Replacing the prior output tree is the only non-transactional filesystem boundary; a final rename failure can leave no output tree.
