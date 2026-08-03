use aster::{BuildSession, Project};

use crate::common::{install_content_adapter, project};

#[test]
fn build_reuses_the_session_and_observes_source_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    let entry = root.join("pages/index.typ");
    std::fs::write(&entry, "#html.elem(\"p\")[first]").unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    driver.build().unwrap();
    let first = std::fs::read_to_string(root.join("dist/index.html")).unwrap();

    driver.build().unwrap();
    let repeated = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert_eq!(repeated, first);

    std::fs::write(&entry, "#html.elem(\"p\")[second]").unwrap();
    driver.build().unwrap();
    let changed = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert_ne!(changed, first);
    assert!(changed.contains("second"));
}

#[test]
fn build_provides_current_date_with_local_and_explicit_offsets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.elem(\"p\")[#datetime.today().display()]\n",
            "#html.elem(\"p\")[#datetime.today(offset: 0).display()]\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let dates = html
        .split(|character: char| !character.is_ascii_digit() && character != '-')
        .filter(|text| text.len() == 10 && text.as_bytes()[4] == b'-' && text.as_bytes()[7] == b'-')
        .collect::<Vec<_>>();
    assert_eq!(dates.len(), 2, "expected two rendered dates in {html}");
}

#[test]
fn build_loads_content_and_frontmatter_through_entry_module() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("content/blog")).unwrap();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    install_content_adapter(root);
    let content_entry = root.join("content/blog/post.typ");
    std::fs::write(
        &content_entry,
        "#metadata((title: \"First\",)) <frontmatter>\n\nFirst body",
    )
    .unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
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
    driver.build().unwrap();
    let first = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(first.contains("First"));
    assert!(first.contains("First body"));

    std::fs::write(
        &content_entry,
        "#metadata((title: \"Second\",)) <frontmatter>\n\nSecond body",
    )
    .unwrap();
    driver.build().unwrap();
    let second = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(second.contains("Second"));
    assert!(second.contains("Second body"));
    assert_ne!(second, first);
}

#[test]
fn reentrant_build_discovers_added_and_removed_pages() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    let index = root.join("pages/index.typ");
    let about = root.join("pages/about.typ");
    std::fs::write(&index, "#html.elem(\"p\")[Index]").unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    driver.build().unwrap();
    assert!(root.join("dist/index.html").is_file());

    std::fs::write(&about, "#html.elem(\"p\")[About]").unwrap();
    driver.build().unwrap();
    assert!(root.join("dist/index.html").is_file());
    assert!(root.join("dist/about/index.html").is_file());

    std::fs::remove_file(index).unwrap();
    driver.build().unwrap();
    assert!(!root.join("dist/index.html").exists());
    assert!(root.join("dist/about/index.html").is_file());
}

#[test]
fn reentrant_build_recovers_after_compilation_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    let entry = root.join("pages/index.typ");
    std::fs::write(&entry, "#html.elem(\"p\")[First]").unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    driver.build().unwrap();

    std::fs::write(&entry, "#let broken =").unwrap();
    assert!(driver.build().is_err());

    std::fs::write(&entry, "#html.elem(\"p\")[Recovered]").unwrap();
    driver.build().unwrap();
    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(html.contains("Recovered"));
}

#[cfg(unix)]
#[test]
fn build_follows_a_pages_directory_symlink_outside_the_project() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    std::fs::write(
        external.path().join("index.typ"),
        "#html.elem(\"p\")[External]",
    )
    .unwrap();
    symlink(external.path(), root.join("pages")).unwrap();

    let project = project(root);
    let outcome = BuildSession::new(project.clone()).build().unwrap();

    assert_eq!(outcome.outputs, vec![root.join("dist/index.html")]);
    assert!(
        std::fs::read_to_string(root.join("dist/index.html"))
            .unwrap()
            .contains("External")
    );
}

#[cfg(unix)]
#[test]
fn build_preserves_a_symlinked_project_root() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let actual = temp.path().join("actual");
    let linked = temp.path().join("linked");
    std::fs::create_dir_all(actual.join("pages")).unwrap();
    std::fs::write(actual.join("aster.toml"), "").unwrap();
    std::fs::write(
        actual.join("pages/index.typ"),
        "#html.elem(\"p\")[Linked root]",
    )
    .unwrap();
    symlink(&actual, &linked).unwrap();

    let project = Project::open(&linked).unwrap();
    let outcome = BuildSession::new(project.clone()).build().unwrap();

    assert_eq!(outcome.outputs, vec![linked.join("dist/index.html")]);
    assert!(
        std::fs::read_to_string(linked.join("dist/index.html"))
            .unwrap()
            .contains("Linked root")
    );
}
