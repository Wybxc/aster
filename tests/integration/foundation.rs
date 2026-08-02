use aster::{BuildSession, FilesystemDependency, Project};

#[test]
fn dependencies_include_accessed_trees_even_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages/blog/nested")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        "[paths]\nsource = \"pages\"\ncontent = \"entries\"\noutput = \"public\"\n",
    )
    .unwrap();
    let project = Project::open(root.to_owned()).unwrap();

    let mut session = BuildSession::new(project.clone());
    assert!(
        !session
            .dependencies()
            .any(|dependency| dependency == FilesystemDependency::File(project.config_file()))
    );
    assert!(
        !session
            .dependencies()
            .any(|dependency| dependency == FilesystemDependency::Tree(root.join("pages")))
    );
    assert!(
        !session
            .dependencies()
            .any(|dependency| dependency == FilesystemDependency::Tree(root.join("entries")))
    );

    session.build().unwrap();
    let dependencies = session.dependencies().collect::<Vec<_>>();

    assert!(dependencies.contains(&FilesystemDependency::File(project.config_file())));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("pages"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("entries"))));
    assert!(!dependencies.contains(&FilesystemDependency::Tree(root.join("src"))));

    std::fs::create_dir(root.join("entries")).unwrap();
    session.build().unwrap();
    assert!(
        session
            .dependencies()
            .any(|dependency| dependency == FilesystemDependency::Tree(root.join("entries")))
    );
}

#[test]
fn dependencies_include_observed_inputs_but_not_generated_outputs() {
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

    let mut session = BuildSession::new(project);
    session.build().unwrap();
    let dependencies = session.dependencies().collect::<Vec<_>>();

    assert!(dependencies.contains(&FilesystemDependency::File(theme)));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("src"))));
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
            .any(|dependency| dependency == FilesystemDependency::Tree(root.join("src"))),
        "a cached build must still record its directory access"
    );
}

#[test]
fn dependency_snapshot_follows_reloaded_layout() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    let project = Project::open(root.to_owned()).unwrap();
    let mut session = BuildSession::new(project);
    session.build().unwrap();

    assert!(
        session
            .dependencies()
            .any(|dependency| dependency == FilesystemDependency::Tree(root.join("src")))
    );

    std::fs::write(
        root.join("aster.toml"),
        "[paths]\nsource = \"pages\"\ncontent = \"entries\"\n",
    )
    .unwrap();
    assert!(session.build().is_err());
    let dependencies = session.dependencies().collect::<Vec<_>>();

    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("pages"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("entries"))));
    assert!(!dependencies.contains(&FilesystemDependency::Tree(root.join("src"))));
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
