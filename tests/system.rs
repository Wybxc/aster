use std::path::Path;
use std::process::{Command, Output};

use aster::{BuildSession, Project};

fn write_tailwind_project(root: &Path) {
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[\n",
            "    #html.elem(\"link\", attrs: (\"rel\": \"tailwind\", \"href\": \"/styles/site.css\"))\n",
            "    #html.elem(\"link\", attrs: (\"rel\": \"css\", \"href\": \"/styles/plain.css\"))\n",
            "  ]\n",
            "  html.body[Page]\n",
            "})\n",
        ),
    )
    .unwrap();
    std::fs::write(root.join("styles/site.css"), "@import \"tailwindcss\";").unwrap();
    std::fs::write(root.join("styles/plain.css"), ".plain-css { color: blue; }").unwrap();
    std::fs::write(root.join("aster.toml"), "[highlight]\nenabled = false\n").unwrap();
}

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
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.elem(\"p\")[Built]").unwrap();
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

#[cfg(unix)]
#[test]
fn build_uses_tailwind_cli_for_tailwind_links() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write_tailwind_project(root);
    let bin = root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let executable = bin.join("tailwindcss");
    std::fs::write(
        &executable,
        "#!/bin/sh\nprintf '.tailwind-generated { color: red; }\\n'\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_aster"))
        .arg("build")
        .arg("--project")
        .arg(root)
        .env("PATH", &bin)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let css = std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find_map(|path| {
            let css = std::fs::read_to_string(path).ok()?;
            css.contains(".tailwind-generated").then_some(css)
        })
        .unwrap();
    assert!(css.contains(".tailwind-generated"), "{css}");
    let plain_css = std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find_map(|path| {
            let css = std::fs::read_to_string(path).ok()?;
            css.contains(".plain-css").then_some(css)
        })
        .unwrap();
    assert!(plain_css.contains(".plain-css"), "{plain_css}");
    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert_eq!(html.matches("rel=\"stylesheet\"").count(), 2, "{html}");
    assert!(!html.contains("rel=\"tailwind\""), "{html}");
    assert!(!html.contains("rel=\"css\""), "{html}");
}

#[test]
fn build_suggests_installing_missing_tailwind_cli() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write_tailwind_project(root);
    let empty_bin = root.join("empty-bin");
    std::fs::create_dir(&empty_bin).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_aster"))
        .arg("build")
        .arg("--project")
        .arg(root)
        .env("PATH", empty_bin)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires the `tailwindcss` executable"),
        "{stderr}"
    );
    assert!(
        stderr.contains("hint: install the standalone Tailwind CSS CLI"),
        "{stderr}"
    );
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

    assert!(destination.join("pages/index.typ").is_file());
    assert!(destination.join("styles/site.css").is_file());
    assert!(destination.join("lib.typ").is_file());
    assert!(destination.join("components/navigation.typ").is_file());
    assert!(destination.join("templates/site.typ").is_file());
    assert!(!destination.join("site.typ").exists());
    assert!(!destination.join("lib/aster/content.typ").exists());
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
