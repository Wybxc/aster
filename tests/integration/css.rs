use aster::{BuildSession, FilesystemDependency};

use crate::common::{generated_asset_containing, project, write_css_page};

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
    driver.build().unwrap();
    let (first_path, first_css) = generated_asset_containing(root, ".page");
    assert!(
        first_path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("style."))
    );
    assert!(first_css.contains(".theme"));
    assert!(first_css.contains(".page"));
    let dependencies = driver.dependencies();
    assert!(dependencies.contains(&FilesystemDependency::File(entry.clone())));
    assert!(dependencies.contains(&FilesystemDependency::File(dependency.clone())));

    std::fs::write(&dependency, ".theme { color: green; }").unwrap();
    driver.build().unwrap();
    let (changed_path, changed_css) = generated_asset_containing(root, ".page");
    assert_ne!(changed_path, first_path);
    assert_ne!(changed_css, first_css);
    assert!(!first_path.exists());
}

#[test]
fn transforms_for_configured_targets_without_minifying() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    write_css_page(root);
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[css]\n",
            "minify = false\n",
            "targets = [\"ie 11\"]\n",
            "custom-media = true\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/style.css"),
        concat!(
            "@custom-media --narrow (max-width: 30rem);\n",
            "@media (--narrow) {\n",
            "  .page { color: red; & .child { color: blue; } }\n",
            "}\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let css = generated_asset_containing(root, ".page").1;
    assert!(css.contains("@media (max-width: 30rem)"), "{css}");
    assert!(!css.contains("@custom-media"), "{css}");
    assert!(css.contains(".page .child"), "{css}");
    assert!(css.contains('\n'), "CSS should not be minified: {css}");
}

#[test]
fn rejects_invalid_browser_targets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.body[Page]").unwrap();
    std::fs::write(
        root.join("aster.toml"),
        "[css]\ntargets = [\"not-a-browser 1\"]\n",
    )
    .unwrap();
    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("invalid Browserslist query must fail the build");

    assert!(
        format!("{error:#}").contains("invalid CSS browser targets"),
        "unexpected error: {error:#}"
    );
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
    let mut driver = BuildSession::new(project.clone());
    assert!(driver.build().is_err());
    let dependencies = driver.dependencies();
    assert!(
        dependencies.contains(&FilesystemDependency::File(missing.clone())),
        "missing {missing:?} in {dependencies:?}"
    );

    std::fs::write(&missing, ".created { color: green; }").unwrap();
    driver.build().unwrap();
    assert!(
        generated_asset_containing(root, ".created")
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
    session.build().unwrap();

    assert!(
        generated_asset_containing(root, ".secret")
            .1
            .contains(".secret")
    );
    assert!(
        session
            .dependencies()
            .into_iter()
            .any(|dependency| dependency == FilesystemDependency::File(root.join("secret.css")))
    );
}

#[test]
fn records_inputs_inside_output_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir(root.join("dist")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("src/style.css"), "@import \"../dist/input.css\";").unwrap();
    let input = root.join("dist/input.css");
    std::fs::write(&input, ".from-output { color: red; }").unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();

    assert!(
        session.dependencies().into_iter().any(
            |dependency| matches!(dependency, FilesystemDependency::File(path) if path == input)
        )
    );
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
    let linked = root.join("src/shared.css");
    let mut session = BuildSession::new(project.clone());
    session.build().unwrap();

    assert!(
        generated_asset_containing(root, ".shared")
            .1
            .contains(".shared")
    );
    assert!(session.dependencies().into_iter().any(
        |dependency| matches!(dependency, FilesystemDependency::File(path) if path == linked)
    ));
}
