use aster::Project;
use typst::syntax::VirtualPath;

#[test]
fn structural_watch_paths_include_nested_and_missing_layout_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src/blog/nested")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    let project = Project::open(root.to_owned()).unwrap();

    let paths = project.watch_paths(&[]);

    assert!(paths.contains(&project.config_file()));
    assert!(paths.contains(&project.src_dir()));
    assert!(paths.contains(&project.src_dir().join("blog")));
    assert!(paths.contains(&project.src_dir().join("blog/nested")));
    assert!(paths.contains(&project.content_dir()));
}

#[test]
fn watch_paths_merge_dependencies_and_exclude_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src/blog")).unwrap();
    std::fs::create_dir_all(root.join("dist")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    let project = Project::open(root.to_owned()).unwrap();
    let theme = root.join("theme.tmTheme");
    let generated = project.output_dir().join("index.html");

    let paths = project.watch_paths(&[theme.clone(), generated.clone()]);

    assert!(paths.contains(&theme));
    assert!(paths.contains(&project.src_dir().join("blog")));
    assert!(!paths.contains(&generated));
    assert!(
        !paths
            .iter()
            .any(|path| VirtualPath::virtualize(&project.output_dir(), path).is_ok())
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
