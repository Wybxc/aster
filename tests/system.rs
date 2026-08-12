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
            "    #html.elem(\"link\", attrs: (\"rel\": \"stylesheet\", \"href\": \"/styles/plain.css\"))\n",
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

fn write_module_project(root: &Path) {
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::write(
        root.join("components/widget.typ"),
        concat!(
            "#let widget() = [\n",
            "  #metadata(\"./entry.js\") <aster-module>\n",
            "  #html.elem(\"div\")[Widget]\n",
            "]\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("components/entry.js"),
        "import { value } from './dependency.js'; console.log(value);",
    )
    .unwrap();
    std::fs::write(
        root.join("components/dependency.js"),
        "export const value = 'dependency';",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("scripts")).unwrap();
    std::fs::write(
        root.join("scripts/entry.js"),
        "import { value } from './dependency.js'; console.log(value);",
    )
    .unwrap();
    std::fs::write(
        root.join("scripts/dependency.js"),
        "export const value = 'page dependency';",
    )
    .unwrap();
    std::fs::write(root.join("scripts/classic.js"), "console.log('classic');").unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#import \"/components/widget.typ\": widget\n",
            "#html.html({\n",
            "  html.head[\n",
            "    #html.elem(\"link\", attrs: (rel: \"modulepreload\", href: \"/scripts/entry.js\"))\n",
            "    #html.elem(\"script\", attrs: (\"type\": \"module\", \"src\": \"/scripts/entry.js?v=1#main\"))\n",
            "    #html.script(\"console.log('inline module source')\", type: \"module\")\n",
            "    #html.elem(\"script\", attrs: (\"type\": \"module\", \"src\": \"https://example.com/remote.js\"))\n",
            "    #html.elem(\"script\", attrs: (\"src\": \"/scripts/classic.js\"))\n",
            "  ]\n",
            "  html.body[#widget()]\n",
            "})\n",
        ),
    )
    .unwrap();
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("INFO  built 1 page in "),
        "missing build outcome in {stderr}"
    );
    for stage in [
        "configured project",
        "loaded fonts",
        "prepared build",
        "planned routes",
        "rendered pages",
        "ran generators",
        "published output",
    ] {
        assert!(
            stderr.contains(&format!("INFO  {stage} in ")),
            "missing {stage} timing in {stderr}"
        );
    }
    assert!(
        stderr.contains("INFO    rendered page / in "),
        "missing indented page timing in {stderr}"
    );
    assert!(
        !stderr.contains("compiled /pages/index.typ"),
        "default output included verbose details in {stderr}"
    );
    for implementation_detail in [" new", " close", "time.busy", "time.idle"] {
        assert!(
            !stderr.contains(implementation_detail),
            "output exposed {implementation_detail} in {stderr}"
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_aster"))
        .arg("build")
        .arg("-v")
        .arg("--project")
        .arg(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for detail in [
        "DEBUG   configured project ",
        "DEBUG   discovered ",
        "DEBUG   collected 0 public files",
        "DEBUG   loaded 0 content entries",
        "DEBUG   planned 1 page",
        "DEBUG   planned 0 generators",
        "INFO    rendered page / in ",
        "DEBUG     compiled /pages/index.typ in ",
        "DEBUG     transformed document in ",
        "DEBUG     encoded HTML in ",
        "DEBUG   staging 1 file (",
    ] {
        assert!(
            stderr.contains(detail),
            "missing {detail} detail in {stderr}"
        );
    }
    for implementation_detail in [" new", " close", "time.busy", "time.idle"] {
        assert!(
            !stderr.contains(implementation_detail),
            "output exposed {implementation_detail} in {stderr}"
        );
    }
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

#[cfg(unix)]
#[test]
fn build_uses_esbuild_for_module_resources_and_html_scripts() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write_module_project(root);
    let bin = root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let executable = bin.join("esbuild");
    std::fs::write(
        &executable,
        r#"#!/bin/sh
outfile=
metafile=
entry=
for argument in "$@"; do
  case "$argument" in
    --outfile=*) outfile=${argument#--outfile=} ;;
    --metafile=*) metafile=${argument#--metafile=} ;;
    -*) ;;
    *) entry=$argument ;;
  esac
done
printf 'console.log("bundled dependency");\n' > "$outfile"
if [ -z "$entry" ]; then
  printf '{"inputs":{"<stdin>":{}},"outputs":{"%s":{}}}\n' "$outfile" > "$metafile"
else
  printf '{"inputs":{"entry.js":{},"dependency.js":{}},"outputs":{"%s":{}}}\n' "$outfile" > "$metafile"
fi
"#,
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
    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(html.contains("type=\"module\""), "{html}");
    assert!(!html.contains("defer"), "{html}");
    assert!(!html.contains("src=\"/scripts/entry.js"), "{html}");
    assert!(!html.contains("inline module source"), "{html}");
    assert!(
        html.contains("src=\"https://example.com/remote.js\""),
        "{html}"
    );
    assert!(!html.contains("src=\"/scripts/classic.js\""), "{html}");
    assert!(html.contains("?v=1#main\""), "{html}");
    assert!(html.contains("rel=\"modulepreload\""), "{html}");
    let classic = std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find_map(|path| {
            let script = std::fs::read_to_string(path).ok()?;
            script.contains("console.log('classic')").then_some(script)
        })
        .expect("published classic script");
    assert!(classic.contains("console.log('classic')"));
    let script = std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find_map(|path| {
            let script = std::fs::read_to_string(path).ok()?;
            script.contains("bundled dependency").then_some(script)
        })
        .expect("bundled module asset");
    assert!(script.contains("bundled dependency"));
}

#[test]
fn build_suggests_installing_missing_esbuild_cli() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write_module_project(root);
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
        stderr.contains("requires the `esbuild` executable"),
        "{stderr}"
    );
    assert!(stderr.contains("hint: install esbuild"), "{stderr}");
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
    assert_eq!(outcome.pages.len(), 1);
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
