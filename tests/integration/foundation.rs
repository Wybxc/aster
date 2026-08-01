use aster::foundation::config::AsterConfig;
use aster::foundation::project::ProjectRoot;
use typst::foundations::{Str, Value};

#[test]
fn loads_typst_inputs_and_highlight_config() {
    let temp = tempfile::tempdir().unwrap();
    let config_file = temp.path().join("aster.toml");
    std::fs::write(
        &config_file,
        concat!(
            "title = \"Aster\"\n",
            "published = 1979-05-27T07:32:00Z\n",
            "[site]\n",
            "enabled = true\n",
            "[highlight.themes]\n",
            "light = \"Solarized (light)\"\n",
            "dark = \"Solarized (dark)\"\n",
        ),
    )
    .unwrap();

    let config = AsterConfig::load(&config_file).unwrap();

    assert_eq!(
        config.dict.get("title").unwrap(),
        &Value::Str(Str::from("Aster"))
    );
    assert_eq!(
        config.dict.get("published").unwrap(),
        &Value::Str(Str::from("1979-05-27T07:32:00Z"))
    );
    let Value::Dict(site) = config.dict.get("site").unwrap() else {
        panic!("site must be a dictionary");
    };
    assert_eq!(site.get("enabled").unwrap(), &Value::Bool(true));
    assert_eq!(config.highlight.themes.light, "Solarized (light)");
    assert_eq!(config.highlight.themes.dark, "Solarized (dark)");
}

#[test]
fn structural_watch_paths_include_nested_and_missing_layout_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src/blog/nested")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    let project = ProjectRoot::new(root.to_owned()).unwrap();

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
    let project = ProjectRoot::new(root.to_owned()).unwrap();
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
