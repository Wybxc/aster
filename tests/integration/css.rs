use aster::build::pipeline::BuildDriver;
use aster::foundation::config::AsterConfig;

use crate::common::{build, generated_asset, project, write_css_page};

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
    let mut driver = BuildDriver::new(project.clone());
    build(&mut driver, &project);
    let (first_path, first_css) = generated_asset(&project, "css.");
    assert!(first_css.contains(".theme"));
    assert!(first_css.contains(".page"));
    let dependencies = driver.dependencies();
    assert!(dependencies.contains(&std::fs::canonicalize(&entry).unwrap()));
    assert!(dependencies.contains(&std::fs::canonicalize(&dependency).unwrap()));

    build(&mut driver, &project);
    assert_eq!(
        generated_asset(&project, "css."),
        (first_path.clone(), first_css.clone())
    );

    std::fs::write(root.join("src/unrelated.css"), ".unused { color: black; }").unwrap();
    build(&mut driver, &project);
    assert_eq!(
        generated_asset(&project, "css."),
        (first_path.clone(), first_css.clone())
    );

    std::fs::write(&dependency, ".theme { color: green; }").unwrap();
    build(&mut driver, &project);
    let (changed_path, changed_css) = generated_asset(&project, "css.");
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
    let tracked_missing = std::fs::canonicalize(root.join("src"))
        .unwrap()
        .join("missing.css");

    let project = project(root);
    let mut driver = BuildDriver::new(project.clone());
    assert!(
        driver
            .build(AsterConfig::load(&project.config_file()).unwrap())
            .is_err()
    );
    assert!(
        driver
            .build(AsterConfig::load(&project.config_file()).unwrap())
            .is_err()
    );
    let dependencies = driver.dependencies();
    assert!(
        dependencies.contains(&tracked_missing),
        "missing {tracked_missing:?} in {dependencies:?}"
    );

    std::fs::write(&missing, ".created { color: green; }").unwrap();
    build(&mut driver, &project);
    assert!(generated_asset(&project, "css.").1.contains(".created"));
}

#[test]
fn rejects_transitive_import_outside_source_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("src/style.css"), "@import \"../secret.css\";").unwrap();
    std::fs::write(root.join("secret.css"), ".secret { color: red; }").unwrap();

    let project = project(root);
    let error = BuildDriver::new(project.clone())
        .build(AsterConfig::load(&project.config_file()).unwrap())
        .err()
        .expect("escaping import must fail");

    assert!(format!("{error:#}").contains("escapes"));
}
