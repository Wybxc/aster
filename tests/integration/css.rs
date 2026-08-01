use aster::BuildSession;

use crate::common::{build, generated_asset_containing, project, write_css_page};

#[test]
fn bundles_and_tracks_entry_and_transitive_imports() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    write_css_page(root);
    let entry = root.join("src/style.css");
    let dependency = root.join("src/theme.css");
    std::fs::write(&entry, "@import \"theme.css\"; .page { color: red; }").unwrap();
    std::fs::write(&dependency, ".theme { color: blue; }").unwrap();

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    build(&mut driver);
    let (first_path, first_css) = generated_asset_containing(&project, ".page");
    assert!(
        first_path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("style."))
    );
    assert!(first_css.contains(".theme"));
    assert!(first_css.contains(".page"));
    let dependencies = driver.dependencies();
    assert!(dependencies.contains(&entry));
    assert!(dependencies.contains(&dependency));

    build(&mut driver);
    assert_eq!(
        generated_asset_containing(&project, ".page"),
        (first_path.clone(), first_css.clone())
    );

    std::fs::write(root.join("src/unrelated.css"), ".unused { color: black; }").unwrap();
    build(&mut driver);
    assert_eq!(
        generated_asset_containing(&project, ".page"),
        (first_path.clone(), first_css.clone())
    );

    std::fs::write(&dependency, ".theme { color: green; }").unwrap();
    build(&mut driver);
    let (changed_path, changed_css) = generated_asset_containing(&project, ".page");
    assert_ne!(changed_path, first_path);
    assert_ne!(changed_css, first_css);
    assert!(!first_path.exists());
}

#[test]
fn rechecks_missing_imports() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("src/style.css"), "@import \"missing.css\";").unwrap();
    let missing = root.join("src/missing.css");

    let project = project(root);
    let tracked_missing = project.src_dir().join("missing.css");
    let mut driver = BuildSession::new(project.clone());
    assert!(driver.build().is_err());
    assert!(driver.build().is_err());
    let dependencies = driver.dependencies();
    assert!(
        dependencies.contains(&tracked_missing),
        "missing {tracked_missing:?} in {dependencies:?}"
    );

    std::fs::write(&missing, ".created { color: green; }").unwrap();
    build(&mut driver);
    assert!(
        generated_asset_containing(&project, ".created")
            .1
            .contains(".created")
    );
}

#[test]
fn allows_transitive_import_outside_source_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("src/style.css"), "@import \"../secret.css\";").unwrap();
    std::fs::write(root.join("secret.css"), ".secret { color: red; }").unwrap();

    let project = project(root);
    let mut session = BuildSession::new(project.clone());
    build(&mut session);

    assert!(
        generated_asset_containing(&project, ".secret")
            .1
            .contains(".secret")
    );
    assert!(session.dependencies().contains(&root.join("secret.css")));
}

#[test]
fn rejects_transitive_import_outside_project_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("src/style.css"), "@import \"../../secret.css\";").unwrap();

    let project = project(root);
    let error = BuildSession::new(project)
        .build()
        .err()
        .expect("escaping import must fail");

    assert!(format!("{error:#}").contains("escapes project root"));
}

#[cfg(unix)]
#[test]
fn allows_symlinked_css_outside_project_root() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("src/style.css"), "@import \"shared.css\";").unwrap();
    std::fs::write(
        external.path().join("shared.css"),
        ".shared { color: green; }",
    )
    .unwrap();
    let linked = root.join("src/shared.css");
    symlink(external.path().join("shared.css"), &linked).unwrap();

    let project = project(root);
    let linked = project.src_dir().join("shared.css");
    let mut session = BuildSession::new(project.clone());
    build(&mut session);

    assert!(
        generated_asset_containing(&project, ".shared")
            .1
            .contains(".shared")
    );
    assert!(session.dependencies().contains(&linked));
}
