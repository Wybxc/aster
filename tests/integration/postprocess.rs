use aster::{BuildSession, FilesystemDependency};

use crate::common::project;

#[cfg(unix)]
#[test]
fn postprocessor_imports_only_its_private_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.p[Page]").unwrap();
    std::fs::write(root.join("search.toml"), "language = 'en'").unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[[postprocess]]\n",
            "name = \"search\"\n",
            "command = [\"sh\", \"-c\", \"printf 'index' > \\\"$2/search.js\\\"\", \"aster\", \"{site}\", \"{output}\"]\n",
            "mount = \"pagefind\"\n",
            "watch = [\"search.toml\"]\n",
        ),
    )
    .unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();

    assert_eq!(
        std::fs::read_to_string(root.join("dist/pagefind/search.js")).unwrap(),
        "index"
    );
    assert!(root.join("dist/index.html").is_file());
    assert!(
        session
            .dependencies()
            .contains(&FilesystemDependency::File(root.join("search.toml")))
    );
}

#[cfg(unix)]
#[test]
fn postprocessor_can_mutate_the_staged_site() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.p[Original]").unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[[postprocess]]\n",
            "name = \"mutator\"\n",
            "command = [\"sh\", \"-c\", \"printf 'changed' > \\\"$1/index.html\\\"\", \"aster\", \"{site}\"]\n",
        ),
    )
    .unwrap();

    session.build().unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("dist/index.html")).unwrap(),
        "changed"
    );
}

#[cfg(unix)]
#[test]
fn failed_postprocessor_preserves_the_previous_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.p[Original]").unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();
    let original = std::fs::read(root.join("dist/index.html")).unwrap();

    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[[postprocess]]\n",
            "name = \"failure\"\n",
            "command = [\"sh\", \"-c\", \"printf 'changed' > \\\"$1/index.html\\\"; exit 1\", \"aster\", \"{site}\"]\n",
        ),
    )
    .unwrap();

    assert!(session.build().is_err());
    assert_eq!(
        std::fs::read(root.join("dist/index.html")).unwrap(),
        original
    );
}

#[cfg(unix)]
#[test]
fn removed_page_is_not_reported_after_postprocessing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.p[Page]").unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[[postprocess]]\n",
            "name = \"remove-page\"\n",
            "command = [\"sh\", \"-c\", \"rm \\\"$1/index.html\\\"\", \"aster\", \"{site}\"]\n",
        ),
    )
    .unwrap();

    let outcome = BuildSession::new(project(root)).build().unwrap();

    assert!(outcome.pages.is_empty());
    assert!(!root.join("dist/index.html").exists());
}
