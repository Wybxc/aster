use std::path::Path;
use std::process::{Command, Output};

use aster::{BuildSession, Project};

fn init(destination: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aster"))
        .arg("init")
        .arg(destination)
        .output()
        .unwrap()
}

#[test]
fn build_command_builds_the_selected_project() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.elem(\"p\")[Built]").unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_aster"))
        .arg("build")
        .arg("--project")
        .arg(root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.join("dist/index.html").is_file());
    assert!(String::from_utf8_lossy(&output.stderr).contains("built 1 page"));
}

#[test]
fn init_creates_a_buildable_project() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("my-site");

    let output = init(&destination);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(destination.join("src/index.typ").is_file());
    assert!(destination.join("lib/aster/content.typ").is_file());
    assert!(!destination.join("lib").is_symlink());
    let config = std::fs::read_to_string(destination.join("aster.toml")).unwrap();
    assert!(config.contains("name = \"my-site\""));

    let project = Project::open(destination).unwrap();
    let outcome = BuildSession::new(project).build().unwrap();
    assert_eq!(outcome.outputs.len(), 1);
}

#[test]
fn init_accepts_an_existing_empty_directory() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("empty");
    std::fs::create_dir(&destination).unwrap();

    let output = init(&destination);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(destination.join("aster.toml").is_file());
}

#[test]
fn init_refuses_to_overwrite_a_nonempty_directory() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("existing");
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("keep.txt"), "keep").unwrap();

    let output = init(&destination);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("is not empty"));
    assert_eq!(
        std::fs::read_to_string(destination.join("keep.txt")).unwrap(),
        "keep"
    );
    assert!(!destination.join("aster.toml").exists());
}
