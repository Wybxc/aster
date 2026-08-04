use aster::{BuildSession, FilesystemDependency};

use crate::common::{generated_asset_containing, project, write_css_page};

#[test]
fn bundles_and_tracks_entry_and_transitive_imports() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    let entry = root.join("styles/style.css");
    let dependency = root.join("styles/theme.css");
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
fn publishes_and_tracks_assets_from_transitive_stylesheets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles/theme")).unwrap();
    std::fs::create_dir_all(root.join("styles/fonts")).unwrap();
    write_css_page(root);
    let entry = root.join("styles/style.css");
    let imported = root.join("styles/theme/fonts.css");
    let font = root.join("styles/fonts/site.woff2");
    std::fs::write(&entry, "@import \"theme/fonts.css\";").unwrap();
    std::fs::write(
        &imported,
        concat!(
            "@font-face {",
            "font-family: Site;",
            "src: url(\"../fonts/site.woff2?v=1#regular\") format(\"woff2\");",
            "}",
            ".font-rule {",
            "font-family: Site;",
            "background: url(\"../fonts/site.woff2?v=1#regular\");",
            "}",
        ),
    )
    .unwrap();
    std::fs::write(&font, b"first font").unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();

    let first_font = generated_asset_with_extension(root, "woff2");
    assert_eq!(std::fs::read(&first_font).unwrap(), b"first font");
    let (first_css_path, first_css) = generated_asset_containing(root, ".font-rule");
    let first_font_name = first_font.file_name().unwrap().to_string_lossy();
    assert!(first_font_name.starts_with("site."), "{first_font_name}");
    let rewritten_url = format!("url(\"{first_font_name}?v=1#regular\")");
    assert_eq!(first_css.matches(&rewritten_url).count(), 2, "{first_css}");
    assert!(!first_css.contains("../fonts/site.woff2"), "{first_css}");
    let dependencies = session.dependencies();
    assert!(dependencies.contains(&FilesystemDependency::File(entry)));
    assert!(dependencies.contains(&FilesystemDependency::File(imported)));
    assert!(dependencies.contains(&FilesystemDependency::File(font.clone())));

    std::fs::write(&font, b"changed font").unwrap();
    session.build().unwrap();

    let changed_font = generated_asset_with_extension(root, "woff2");
    let (changed_css_path, changed_css) = generated_asset_containing(root, ".font-rule");
    assert_ne!(changed_font, first_font);
    assert_ne!(changed_css_path, first_css_path);
    assert_ne!(changed_css, first_css);
    assert!(!first_font.exists());
    assert!(!first_css_path.exists());
}

#[test]
fn reuses_resolved_stylesheet_across_page_output_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    let template = std::fs::read_to_string(root.join("pages/index.typ")).unwrap();
    std::fs::create_dir_all(root.join("pages/nested")).unwrap();
    std::fs::write(root.join("pages/nested/index.typ"), template).unwrap();
    std::fs::write(
        root.join("styles/style.css"),
        ".shared { background: url(\"pixel.bin\"); }",
    )
    .unwrap();
    std::fs::write(root.join("styles/pixel.bin"), b"pixel").unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let (css_path, css) = generated_asset_containing(root, ".shared");
    let css_name = css_path.file_name().unwrap().to_string_lossy();
    let root_html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let nested_html = std::fs::read_to_string(root.join("dist/nested/index.html")).unwrap();
    assert!(root_html.contains(&format!("href=\"_assets/{css_name}\"")));
    assert!(nested_html.contains(&format!("href=\"../_assets/{css_name}\"")));
    assert!(css.contains("pixel."), "{css}");
    assert!(!css.contains("pixel.bin"), "{css}");
}

#[test]
fn preserves_browser_managed_urls() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(
        root.join("styles/style.css"),
        concat!(
            "@import \"https://example.com/theme.css\";",
            ".remote { background: url(\"https://example.com/image.png\"); }",
            ".root { background: url(\"/images/site.svg\"); }",
            ".inline { background: url(\"data:image/svg+xml,%3Csvg/%3E\"); }",
            ".fragment { filter: url(\"#blur\"); }",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let css = generated_asset_containing(root, ".remote").1;
    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(
        html.contains("<link rel=\"stylesheet\" href=\"/site.css\">"),
        "{html}"
    );
    assert!(css.contains("https://example.com/theme.css"), "{css}");
    assert!(css.contains("https://example.com/image.png"), "{css}");
    assert!(css.contains("/images/site.svg"), "{css}");
    assert!(css.contains("data:image/svg+xml,%3Csvg/%3E"), "{css}");
    assert!(css.contains("#blur"), "{css}");
}

#[test]
fn rechecks_missing_css_assets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(
        root.join("styles/style.css"),
        ".asset-rule { background: url(\"assets/missing.bin\"); }",
    )
    .unwrap();
    let missing = root.join("styles/assets/missing.bin");

    let mut session = BuildSession::new(project(root));
    assert!(session.build().is_err());
    assert!(
        session
            .dependencies()
            .contains(&FilesystemDependency::File(missing.clone()))
    );

    std::fs::create_dir_all(missing.parent().unwrap()).unwrap();
    std::fs::write(&missing, b"asset").unwrap();
    session.build().unwrap();
    assert_eq!(
        std::fs::read(generated_asset_with_extension(root, "bin")).unwrap(),
        b"asset"
    );
}

#[test]
fn rejects_css_assets_outside_project_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(
        root.join("styles/style.css"),
        ".asset-rule { background: url(\"../../outside.bin\"); }",
    )
    .unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("escaping CSS asset must fail");
    assert!(format!("{error:#}").contains("escapes project root"));
}

#[test]
fn transforms_for_configured_targets_without_minifying() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
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
        root.join("styles/style.css"),
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
fn minify_only_compacts_serialized_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(
        root.join("aster.toml"),
        "[css]\nminify = false\ntargets = []\n",
    )
    .unwrap();
    std::fs::write(
        root.join("styles/style.css"),
        ".page { color: red; color: red; }",
    )
    .unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();

    let readable = generated_asset_containing(root, ".page").1;
    assert_eq!(readable.matches("color: red").count(), 1, "{readable}");
    assert!(readable.contains('\n'), "{readable}");

    std::fs::write(
        root.join("aster.toml"),
        "[css]\nminify = true\ntargets = []\n",
    )
    .unwrap();
    session.build().unwrap();

    let compact = generated_asset_containing(root, ".page").1;
    assert_eq!(compact, ".page{color:red}");
}

#[test]
fn rejects_invalid_browser_targets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.body[Page]").unwrap();
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
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("styles/style.css"), "@import \"missing.css\";").unwrap();
    let missing = root.join("styles/missing.css");

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
fn allows_imports_across_project_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("styles/style.css"), "@import \"../secret.css\";").unwrap();
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
    std::fs::create_dir_all(root.join("styles")).unwrap();
    std::fs::create_dir(root.join("dist")).unwrap();
    write_css_page(root);
    std::fs::write(
        root.join("styles/style.css"),
        "@import \"../dist/input.css\";",
    )
    .unwrap();
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
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(
        root.join("styles/style.css"),
        "@import \"../../secret.css\";",
    )
    .unwrap();

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
    std::fs::create_dir_all(root.join("styles")).unwrap();
    write_css_page(root);
    std::fs::write(root.join("styles/style.css"), "@import \"shared.css\";").unwrap();
    std::fs::write(
        external.path().join("shared.css"),
        ".shared { color: green; }",
    )
    .unwrap();
    let linked = root.join("styles/shared.css");
    symlink(external.path().join("shared.css"), &linked).unwrap();

    let project = project(root);
    let linked = root.join("styles/shared.css");
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

fn generated_asset_with_extension(root: &std::path::Path, extension: &str) -> std::path::PathBuf {
    std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|value| value == extension))
        .unwrap()
}
