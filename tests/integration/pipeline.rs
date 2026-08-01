use aster::BuildSession;

use crate::common::{build, install_content_adapter, project};

#[test]
fn build_reuses_the_session_and_observes_source_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    let entry = root.join("src/index.typ");
    std::fs::write(&entry, "#html.elem(\"p\")[first]").unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    build(&mut driver);
    let first = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();

    build(&mut driver);
    let repeated = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();
    assert_eq!(repeated, first);

    std::fs::write(&entry, "#html.elem(\"p\")[second]").unwrap();
    build(&mut driver);
    let changed = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();
    assert_ne!(changed, first);
    assert!(changed.contains("second"));
}

#[test]
fn build_loads_content_and_frontmatter_through_entry_module() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("content/blog")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    install_content_adapter(root);
    let content_entry = root.join("content/blog/post.typ");
    std::fs::write(
        &content_entry,
        "#metadata((title: \"First\",)) <frontmatter>\n\nFirst body",
    )
    .unwrap();
    std::fs::write(
        root.join("src/index.typ"),
        concat!(
            "#import \"/lib/aster/content.typ\": get-entry\n",
            "#let post = get-entry(\"blog\", \"post\")\n",
            "#let rendered = post.render()\n",
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#rendered.metadata.title #rendered.content]\n",
            "})\n",
        ),
    )
    .unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    build(&mut driver);
    let first = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();
    assert!(first.contains("First"));
    assert!(first.contains("First body"));

    std::fs::write(
        &content_entry,
        "#metadata((title: \"Second\",)) <frontmatter>\n\nSecond body",
    )
    .unwrap();
    build(&mut driver);
    let second = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();
    assert!(second.contains("Second"));
    assert!(second.contains("Second body"));
    assert_ne!(second, first);
}

#[test]
fn reentrant_build_discovers_added_and_removed_pages() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    let index = root.join("src/index.typ");
    let about = root.join("src/about.typ");
    std::fs::write(&index, "#html.elem(\"p\")[Index]").unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    build(&mut driver);
    assert!(project.output_dir().join("index.html").is_file());

    std::fs::write(&about, "#html.elem(\"p\")[About]").unwrap();
    build(&mut driver);
    assert!(project.output_dir().join("index.html").is_file());
    assert!(project.output_dir().join("about.html").is_file());

    std::fs::remove_file(index).unwrap();
    build(&mut driver);
    assert!(!project.output_dir().join("index.html").exists());
    assert!(project.output_dir().join("about.html").is_file());
}

#[test]
fn reentrant_build_recovers_after_compilation_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    let entry = root.join("src/index.typ");
    std::fs::write(&entry, "#html.elem(\"p\")[First]").unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    build(&mut driver);

    std::fs::write(&entry, "#let broken =").unwrap();
    assert!(driver.build().is_err());

    std::fs::write(&entry, "#html.elem(\"p\")[Recovered]").unwrap();
    build(&mut driver);
    let html = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();
    assert!(html.contains("Recovered"));
}
