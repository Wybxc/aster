use aster::{BuildSession, FilesystemDependency, Project};

#[test]
fn includes_accessed_trees_even_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("routes/blog/nested")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[paths]\n",
            "pages = \"routes\"\n",
            "content = \"entries\"\n",
            "public = \"files\"\n",
            "output = \"public\"\n",
        ),
    )
    .unwrap();
    let project = Project::open(root.to_owned()).unwrap();
    let mut session = BuildSession::new(project.clone());

    session.build().unwrap();
    let dependencies = session.dependencies();

    assert!(dependencies.contains(&FilesystemDependency::File(project.config_file())));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("routes"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("entries"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("files"))));
    assert!(!dependencies.contains(&FilesystemDependency::Tree(root.join("pages"))));
}

#[test]
fn includes_observed_inputs_but_not_generated_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages/blog")).unwrap();
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
    std::fs::write(root.join("pages/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    let project = Project::open(root.to_owned()).unwrap();
    let theme = root.join("theme.tmTheme");
    let generated = root.join("dist/index.html");

    let mut session = BuildSession::new(project);
    session.build().unwrap();
    let dependencies = session.dependencies();

    assert!(dependencies.contains(&FilesystemDependency::File(theme)));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("pages"))));
    assert!(!dependencies.contains(&FilesystemDependency::File(generated)));
    assert!(
        !dependencies
            .iter()
            .any(|dependency| dependency.path().starts_with(root.join("dist")))
    );

    session.build().unwrap();
    assert!(
        session
            .dependencies()
            .into_iter()
            .any(|dependency| dependency == FilesystemDependency::Tree(root.join("pages"))),
        "a cached build must still record its directory access"
    );
}

#[test]
fn snapshot_follows_reloaded_layout() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    let project = Project::open(root.to_owned()).unwrap();
    let mut session = BuildSession::new(project);
    session.build().unwrap();

    assert!(
        session
            .dependencies()
            .into_iter()
            .any(|dependency| dependency == FilesystemDependency::Tree(root.join("pages")))
    );

    std::fs::write(
        root.join("aster.toml"),
        "[paths]\npages = \"routes\"\ncontent = \"entries\"\n",
    )
    .unwrap();
    assert!(session.build().is_err());
    let dependencies = session.dependencies();

    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("routes"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("entries"))));
    assert!(!dependencies.contains(&FilesystemDependency::Tree(root.join("pages"))));
}
