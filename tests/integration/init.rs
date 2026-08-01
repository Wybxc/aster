use aster::build::pipeline::BuildDriver;
use aster::cli::init;
use aster::foundation::config::AsterConfig;
use aster::foundation::project::ProjectRoot;

#[test]
fn initializes_a_buildable_project_with_a_real_library_directory() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("my-site");

    init::run(destination.clone()).unwrap();

    assert!(destination.join("src/index.typ").is_file());
    assert!(destination.join("lib/aster/content.typ").is_file());
    assert!(!destination.join("lib").is_symlink());
    let config = std::fs::read_to_string(destination.join("aster.toml")).unwrap();
    assert!(config.contains("name = \"my-site\""));

    let project = ProjectRoot::new(destination).unwrap();
    let config = AsterConfig::load(&project.config_file()).unwrap();
    let outcome = BuildDriver::new(project).build(config).unwrap();
    assert_eq!(outcome.outputs.len(), 1);
}

#[test]
fn initializes_an_existing_empty_directory() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("empty");
    std::fs::create_dir(&destination).unwrap();

    init::run(destination.clone()).unwrap();

    assert!(destination.join("aster.toml").is_file());
}

#[test]
fn refuses_to_overwrite_a_nonempty_directory() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("existing");
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("keep.txt"), "keep").unwrap();

    let error = init::run(destination.clone()).unwrap_err();

    assert!(error.to_string().contains("is not empty"));
    assert_eq!(
        std::fs::read_to_string(destination.join("keep.txt")).unwrap(),
        "keep"
    );
    assert!(!destination.join("aster.toml").exists());
}
