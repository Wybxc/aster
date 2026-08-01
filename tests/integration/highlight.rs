use std::path::Path;

use aster::build::pipeline::BuildDriver;

use crate::common::{build, generated_asset, project};

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
    let mut driver = BuildDriver::new(project.clone());
    build(&mut driver, &project);
    let (first_path, first_css) = generated_asset(&project, "hl.");
    assert!(
        first_css.contains("#112233"),
        "unexpected highlight CSS: {first_css}"
    );

    build(&mut driver, &project);
    assert_eq!(
        generated_asset(&project, "hl."),
        (first_path.clone(), first_css.clone())
    );

    write_theme(&theme, "#445566");
    build(&mut driver, &project);
    let (changed_path, changed_css) = generated_asset(&project, "hl.");
    assert_ne!(changed_path, first_path);
    assert_ne!(changed_css, first_css);
    assert!(changed_css.contains("#445566"));
    assert!(!first_path.exists());
}
