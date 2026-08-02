use aster::{BuildSession, Project};
use typst::syntax::VirtualPath;

#[test]
fn structural_watch_paths_include_nested_and_missing_layout_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages/blog/nested")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        "[paths]\nsource = \"pages\"\ncontent = \"entries\"\noutput = \"public\"\n",
    )
    .unwrap();
    let project = Project::open(root.to_owned()).unwrap();

    let mut session = BuildSession::new(project.clone()).unwrap();
    let paths = session.watch_paths();

    assert!(paths.contains(&project.config_file()));
    assert!(paths.contains(&root.join("pages")));
    assert!(paths.contains(&root.join("pages/blog")));
    assert!(paths.contains(&root.join("pages/blog/nested")));
    assert!(paths.contains(&root.join("entries")));
    assert!(!paths.contains(&root.join("src")));
}

#[test]
fn watch_paths_merge_dependencies_and_exclude_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src/blog")).unwrap();
    std::fs::create_dir_all(root.join("dist")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[highlight.themes]\n",
            "light = \"theme.tmTheme\"\n",
            "dark = \"theme.tmTheme\"\n",
        ),
    )
    .unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    let project = Project::open(root.to_owned()).unwrap();
    let theme = root.join("theme.tmTheme");
    let generated = root.join("dist/index.html");

    let mut session = BuildSession::new(project).unwrap();
    session.build().unwrap();
    let paths = session.watch_paths();

    assert!(paths.contains(&theme));
    assert!(paths.contains(&root.join("src/blog")));
    assert!(!paths.contains(&generated));
    assert!(
        !paths
            .iter()
            .any(|path| VirtualPath::virtualize(&root.join("dist"), path).is_ok())
    );
}

#[cfg(unix)]
#[test]
fn project_root_preserves_a_symbolic_link() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let actual = temp.path().join("actual");
    let linked = temp.path().join("linked");
    std::fs::create_dir(&actual).unwrap();
    std::fs::write(actual.join("aster.toml"), "").unwrap();
    symlink(&actual, &linked).unwrap();

    let project = Project::open(&linked).unwrap();

    assert_eq!(project.root(), std::path::absolute(&linked).unwrap());
    assert!(
        std::fs::symlink_metadata(project.root())
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
