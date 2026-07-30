#metadata((
  title: "Tracked imports make incremental builds predictable.",
  date: "2026-07-30",
  summary: "Tracked Typst imports allow Aster to rebuild only the pages whose inputs have changed.",
  tags: ("comemo", "watch", "architecture"),
)) <frontmatter>

= Tracked imports make incremental builds predictable.

Aster keeps one Typst session alive while watching a project. Each page
compilation records the files it actually reads, including content loaded by an
entry's `render` closure.

== The page is the useful unit of work.

A build still plans the complete output tree. The expensive page compilation is
memoized independently, so an unchanged page can be reused while publication
remains deterministic.

```rust
#[comemo::memoize]
fn compile_page(world: Tracked<dyn World>, output: &Path) -> HtmlDocument {
    typst::compile(&*world)
}
```

If this article changes, a listing that reads its metadata and its detail page
are invalidated. A page that never calls this entry's `render` function stays
cached.

== Lazy entries preserve Typst's dependency tracking.

Collection discovery creates a small module with `id`, `collection`, and
`render`. The module captures a Typst path, but does not eagerly evaluate the
entry. That keeps dependency tracking inside Typst's normal world interface.

#html.elem("aside")[
  You can observe this behavior by running `aster watch`, editing this file,
  and comparing the emitted `build` lines with the complete page count.
]
