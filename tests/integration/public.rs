use aster::{BuildSession, FilesystemDependency};

use crate::common::project;

#[test]
fn copies_public_tree_and_removes_stale_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("public/images")).unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    std::fs::write(root.join("public/CNAME"), "example.com\n").unwrap();
    std::fs::write(root.join("public/images/logo.bin"), [0, 1, 2, 255]).unwrap();

    let mut session = BuildSession::new(project(root));
    let outcome = session.build().unwrap();

    assert_eq!(outcome.outputs, [root.join("dist/index.html")]);
    assert_eq!(
        std::fs::read_to_string(root.join("dist/CNAME")).unwrap(),
        "example.com\n"
    );
    assert_eq!(
        std::fs::read(root.join("dist/images/logo.bin")).unwrap(),
        [0, 1, 2, 255]
    );
    let dependencies = session.dependencies();
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("public"))));
    assert!(dependencies.contains(&FilesystemDependency::File(root.join("public/CNAME"))));
    assert!(dependencies.contains(&FilesystemDependency::File(
        root.join("public/images/logo.bin")
    )));

    std::fs::remove_file(root.join("public/CNAME")).unwrap();
    std::fs::write(root.join("public/images/logo.bin"), [3, 4, 5]).unwrap();
    session.build().unwrap();

    assert!(!root.join("dist/CNAME").exists());
    assert_eq!(
        std::fs::read(root.join("dist/images/logo.bin")).unwrap(),
        [3, 4, 5]
    );
}

#[test]
fn rejects_public_file_that_collides_with_generated_page() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("public")).unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.elem(\"p\")[Generated]").unwrap();
    std::fs::write(root.join("public/index.html"), "Public").unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("public file and page must not share an output path");

    assert!(
        format!("{error:#}").contains("same output path index.html"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn rejects_output_that_overlaps_public_without_deleting_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("public")).unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    let public_file = root.join("public/keep.txt");
    std::fs::write(&public_file, "Keep").unwrap();
    std::fs::write(root.join("aster.toml"), "[paths]\noutput = \"public\"\n").unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("overlapping public and output directories must fail");

    assert!(
        format!("{error:#}").contains("public and output directories must not overlap"),
        "unexpected error: {error:#}"
    );
    assert!(public_file.is_file());
}
