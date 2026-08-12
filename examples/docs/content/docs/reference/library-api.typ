#import "/components/content.typ": callout

#metadata((
  title: "Rust Library API",
  description: "Embed Aster builds and consume outcomes and filesystem dependencies.",
  section: "Reference",
  section_order: 30,
  order: 50,
)) <aster-frontmatter>

The `aster` crate exports the project and build-session interface at its root:

```rust
use aster::{BuildSession, Project};

fn build(root: &std::path::Path) -> anyhow::Result<()> {
    let project = Project::open(root)?;
    let mut session = BuildSession::new(project);
    let outcome = session.build()?;

    println!("published {} pages", outcome.pages.len());
    Ok(())
}
```

= Project discovery

`Project::open(path)` requires `path/aster.toml`. `Project::find(path)` searches
that directory and its ancestors for the nearest manifest. The stored root is
made absolute lexically without canonicalizing symlinks.

= Reusable sessions

A `BuildSession` belongs to one project and is intended to survive repeated
builds. It retains fonts, package state, tracked files, and memoized compilation
inputs while reloading `aster.toml` for every attempt.

`build()` publishes the site and returns a `BuildOutcome` containing:

- `output_dir`: the complete published tree;
- `pages`: authored HTML page paths in route order;
- `generated`: authored generator output paths in route order;
- `warnings`: nonfatal `BuildWarning` values;
- `elapsed`: total build and publication duration.

Generated assets, public files, and postprocessor imports are intentionally not
listed as authored outputs.

= Dependency watching

After each build attempt, `session.dependencies()` returns observed
`FilesystemDependency::File` and `FilesystemDependency::Tree` paths, including
missing inputs. A host can replace its complete watch set with this snapshot and
reuse the same session for the next build. Tree dependencies represent recursive
directory membership; file dependencies are nonrecursive.

#callout(kind: "note")[
  The library emits structured `tracing` spans and events but does not install a
  subscriber. Embedders own filtering and presentation. The bundled CLI uses
  `-v` and `-vv` to expose progressively more detailed events.
]
