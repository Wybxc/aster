use std::path::Path;

use aster::BuildSession;

use crate::common::{generated_asset_containing, project};

fn highlight_stylesheet(root: &Path) -> String {
    let path = std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("highlight.")
        })
        .unwrap();
    std::fs::read_to_string(path).unwrap()
}

fn write_theme(path: &Path, color: &str) {
    let theme = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>name</key><string>Aster Test</string>
  <key>settings</key>
  <array>
    <dict>
      <key>settings</key>
      <dict><key>foreground</key><string>{color}</string></dict>
    </dict>
    <dict>
      <key>scope</key><string>keyword.control</string>
      <key>settings</key>
      <dict><key>foreground</key><string>{color}</string></dict>
    </dict>
  </array>
</dict>
</plist>
"#
    );
    std::fs::write(path, theme).unwrap();
}

#[test]
fn custom_theme_changes_replace_the_highlight_stylesheet() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[highlight.themes]\n",
            "light = \"theme.tmTheme\"\n",
            "dark = \"theme.tmTheme\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#raw(\"if true {}\", block: true, lang: \"rust\")]\n",
            "})\n",
        ),
    )
    .unwrap();
    let theme = root.join("theme.tmTheme");
    write_theme(&theme, "#112233");

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    driver.build().unwrap();
    let (first_path, first_css) = generated_asset_containing(root, "#112233");
    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert_eq!(html.matches("<head>").count(), 1);

    write_theme(&theme, "#445566");
    driver.build().unwrap();
    let (changed_path, changed_css) = generated_asset_containing(root, "#445566");
    assert_ne!(changed_path, first_path);
    assert_ne!(changed_css, first_css);
    assert!(!first_path.exists());
}

#[test]
fn creates_head_before_body_for_highlight_stylesheet() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.body[#raw(\"pub fn main() {}\", block: true, lang: \"rust\")]\n",
            "})\n"
        ),
    )
    .unwrap();

    let project = project(root);
    let mut session = BuildSession::new(project.clone());
    session.build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let head = html.find("<head>").expect("generated head");
    let body = html.find("<body>").expect("existing body");
    assert!(head < body);
    assert!(html[head..body].contains("rel=\"stylesheet\""));
    assert!(html[head..body].contains("href=\"_assets/highlight."));
}

#[test]
fn applies_inherited_theme_styles_to_rust_modifiers() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#raw(\"pub fn main() {}\", block: true, lang: \"rust\")]\n",
            "})\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(html.contains("<span class=\"hl-s0\">pub</span>"));
    let css = highlight_stylesheet(root);
    assert!(css.contains(".hl-s0{color:#a71d5d;font-weight:bold}"));
    assert!(css.contains("[data-theme=\"dark\"] .hl-s0{color:#cc99cc;font-weight:normal}"));
}

#[test]
fn invalid_theme_warns_once_for_the_whole_build() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[highlight.themes]\n",
            "light = \"missing.tmTheme\"\n",
            "dark = \"missing.tmTheme\"\n",
        ),
    )
    .unwrap();
    for page in ["index.typ", "about/index.typ"] {
        let path = root.join("pages").join(page);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "#html.html(html.body[Page])").unwrap();
    }

    let project = project(root);
    let outcome = BuildSession::new(project.clone()).build().unwrap();

    assert_eq!(
        outcome
            .warnings
            .iter()
            .filter(|warning| { warning.as_str().contains("failed to resolve highlight CSS") })
            .count(),
        1
    );
    assert!(root.join("dist/index.html").is_file());
    assert!(root.join("dist/about/index.html").is_file());
}

#[cfg(unix)]
#[test]
fn allows_symlinked_theme_outside_project_root() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[highlight.themes]\n",
            "light = \"theme.tmTheme\"\n",
            "dark = \"theme.tmTheme\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#raw(\"if true {}\", block: true, lang: \"rust\")]\n",
            "})\n",
        ),
    )
    .unwrap();
    let external_theme = external.path().join("theme.tmTheme");
    write_theme(&external_theme, "#123456");
    symlink(external_theme, root.join("theme.tmTheme")).unwrap();

    let project = project(root);
    let mut session = BuildSession::new(project.clone());
    session.build().unwrap();

    assert!(
        generated_asset_containing(root, "#123456")
            .1
            .contains("#123456")
    );
}
