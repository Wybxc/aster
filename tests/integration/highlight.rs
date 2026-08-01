use std::path::Path;

use aster::BuildSession;

use crate::common::{build, generated_asset_containing, project};

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
    std::fs::create_dir_all(root.join("src")).unwrap();
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
        root.join("src/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[Page]\n",
            "})\n",
        ),
    )
    .unwrap();
    let theme = root.join("theme.tmTheme");
    write_theme(&theme, "#112233");

    let project = project(root);
    let mut driver = BuildSession::new(project.clone());
    build(&mut driver);
    let (first_path, first_css) = generated_asset_containing(&project, "#112233");
    assert!(
        first_css.contains("#112233"),
        "unexpected highlight CSS: {first_css}"
    );
    let html = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();
    assert_eq!(html.matches("<head>").count(), 1);

    build(&mut driver);
    assert_eq!(
        generated_asset_containing(&project, "#112233"),
        (first_path.clone(), first_css.clone())
    );

    write_theme(&theme, "#445566");
    build(&mut driver);
    let (changed_path, changed_css) = generated_asset_containing(&project, "#445566");
    assert_ne!(changed_path, first_path);
    assert_ne!(changed_css, first_css);
    assert!(changed_css.contains("#445566"));
    assert!(!first_path.exists());
}

#[test]
fn creates_head_before_body_for_highlight_stylesheet() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    std::fs::write(
        root.join("src/index.typ"),
        concat!("#html.html({\n", "  html.body[Page]\n", "})\n"),
    )
    .unwrap();

    let project = project(root);
    let mut session = BuildSession::new(project.clone());
    build(&mut session);

    let html = std::fs::read_to_string(project.output_dir().join("index.html")).unwrap();
    let head = html.find("<head>").expect("generated head");
    let body = html.find("<body>").expect("existing body");
    assert!(head < body);
    assert!(html[head..body].contains("rel=\"stylesheet\""));
    assert!(html[head..body].contains("href=\"_assets/highlight."));
}

#[cfg(unix)]
#[test]
fn allows_symlinked_theme_outside_project_root() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
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
        root.join("src/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[Page]\n",
            "})\n",
        ),
    )
    .unwrap();
    let external_theme = external.path().join("theme.tmTheme");
    write_theme(&external_theme, "#123456");
    symlink(external_theme, root.join("theme.tmTheme")).unwrap();

    let project = project(root);
    let mut session = BuildSession::new(project.clone());
    build(&mut session);

    assert!(
        generated_asset_containing(&project, "#123456")
            .1
            .contains("#123456")
    );
}
