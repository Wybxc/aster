use std::process::Command;

use crate::common::project;

#[test]
fn build_command_builds_the_selected_project() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.elem(\"p\")[Built]").unwrap();
    let project = project(root);

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
    assert!(project.output_dir().join("index.html").is_file());
    assert!(String::from_utf8_lossy(&output.stderr).contains("built 1 page"));
}
