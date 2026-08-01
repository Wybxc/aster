use aster::Project;

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
            .any(|path| path.starts_with(project.output_dir()))
    );
}
