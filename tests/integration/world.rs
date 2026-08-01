use aster::build::world::TypstSession;
use aster::engine::content;
use typst::foundations::Dict;

use crate::common::{install_content_adapter, project};

#[test]
fn page_compilation_is_reused_and_invalidated_by_dependency_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let marker = root.file_name().unwrap().to_string_lossy();
    std::fs::create_dir_all(root.join("src")).unwrap();
    let entry = root.join("src/index.typ");
    let dependency = root.join("src/data.typ");
    std::fs::write(&entry, "#import \"data.typ\": marker\n#let value = marker").unwrap();
    std::fs::write(&dependency, format!("#let marker = \"first-{marker}\"")).unwrap();

    let project = project(root);
    let mut session = TypstSession::new(project);
    let library = session.library(Dict::new());

    session.compile_page(&entry, &library).unwrap();
    assert!(!comemo::testing::last_was_hit());

    session.reset();
    session.compile_page(&entry, &library).unwrap();
    assert!(comemo::testing::last_was_hit());

    std::fs::write(&dependency, format!("#let marker = \"second-{marker}\"")).unwrap();
    session.reset();
    session.compile_page(&entry, &library).unwrap();
    assert!(!comemo::testing::last_was_hit());
}

#[test]
fn dynamic_content_imports_only_invalidate_dependent_pages() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let marker = root.file_name().unwrap().to_string_lossy();
    std::fs::create_dir_all(root.join("content/blog")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    install_content_adapter(root);
    let content_entry = root.join("content/blog/post.typ");
    std::fs::write(&content_entry, format!("= First {marker}")).unwrap();
    let dependent = root.join("src/dependent.typ");
    std::fs::write(
        &dependent,
        concat!(
            "#import \"/lib/aster/content.typ\": get-entry\n",
            "#let post = get-entry(\"blog\", \"post\")\n",
            "#let rendered = post.render()\n",
            "#html.elem(\"p\")[#rendered.content]\n",
        ),
    )
    .unwrap();
    let independent = root.join("src/independent.typ");
    std::fs::write(&independent, format!("#html.elem(\"p\")[{marker}]")).unwrap();

    let project = project(root);
    let mut session = TypstSession::new(project);
    let inputs = content::install(Dict::new(), content::load(&session).unwrap()).unwrap();
    let library = session.library(inputs);
    session.compile_page(&dependent, &library).unwrap();
    session.compile_page(&independent, &library).unwrap();

    std::fs::write(&content_entry, format!("= Second {marker}")).unwrap();
    session.reset();
    let inputs = content::install(Dict::new(), content::load(&session).unwrap()).unwrap();
    let library = session.library(inputs);

    session.compile_page(&independent, &library).unwrap();
    assert!(comemo::testing::last_was_hit());

    let compiled = session.compile_page(&dependent, &library).unwrap();
    assert!(!comemo::testing::last_was_hit());
    let html = typst_html::html(&compiled.document, &typst_html::HtmlOptions::default()).unwrap();
    assert!(html.contains(&format!("Second {marker}")));
}
