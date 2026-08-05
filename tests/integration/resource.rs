use aster::{BuildSession, FilesystemDependency};

use crate::common::project;

#[test]
fn component_file_resources_are_resolved_tracked_and_injected_once() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::write(
        root.join("components/card.typ"),
        concat!(
            "#let card(label) = [\n",
            "  #metadata(\"./card.css\") <aster-style>\n",
            "  #metadata(\"./card.js\") <aster-script>\n",
            "  #html.elem(\"div\", attrs: (class: \"card\"))[#label]\n",
            "]\n",
        ),
    )
    .unwrap();
    let stylesheet = root.join("components/card.css");
    let script = root.join("components/card.js");
    std::fs::write(&stylesheet, ".card { color: red; }").unwrap();
    std::fs::write(&script, "globalThis.cardsReady = true;").unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#import \"/components/card.typ\": card\n",
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#card(\"one\") #card(\"two\")]\n",
            "})\n",
        ),
    )
    .unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let css_name = generated_asset_containing(root, "css", ".card");
    let js_name = generated_asset_containing(root, "js", "cardsReady");
    assert_eq!(html.matches(&css_name).count(), 1, "{html}");
    assert_eq!(html.matches(&js_name).count(), 1, "{html}");
    let head_end = html.find("</head>").expect("generated head");
    let script_position = html.find("<script src=").expect("component script");
    assert!(script_position < head_end, "{html}");
    assert!(
        html[script_position..head_end].contains(" defer>"),
        "{html}"
    );
    assert!(!html[head_end..].contains(&js_name), "{html}");
    assert!(!html.contains("type=\"module\""), "{html}");
    assert_eq!(html.matches("class=\"card\"").count(), 2, "{html}");

    let dependencies = session.dependencies();
    assert!(dependencies.contains(&FilesystemDependency::File(stylesheet)));
    assert!(dependencies.contains(&FilesystemDependency::File(script)));
}

#[test]
fn raw_resources_are_deduplicated_by_component_and_accept_surrounding_whitespace() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::write(root.join("components/pixel.bin"), b"pixel").unwrap();
    let imported = root.join("components/shared.css");
    std::fs::write(&imported, ".from-import { color: blue; }").unwrap();
    std::fs::write(
        root.join("components/a.typ"),
        r#"#let component-a() = [
  #metadata([
    ```css
    @import "./shared.css";
    .same { color: red; background: url("./pixel.bin"); }
    ```
  ]) <aster-style>
  #metadata([
    ```js
    globalThis.sameComponent = true;
    ```
  ]) <aster-script>
  #html.elem("div", attrs: (class: "a"))[A]
]
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("components/b.typ"),
        r#"#let component-b() = [
  #metadata(
    ```css
    @import "./shared.css";
    .same { color: red; background: url("./pixel.bin"); }
    ```
  ) <aster-style>
  #metadata(
    ```js
    globalThis.sameComponent = true;
    ```
  ) <aster-script>
  #html.elem("div", attrs: (class: "b"))[B]
]
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#import \"/components/a.typ\": component-a\n",
            "#import \"/components/b.typ\": component-b\n",
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#component-a() #component-a() #component-b()]\n",
            "})\n",
        ),
    )
    .unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let css = generated_assets(root, "css")
        .into_iter()
        .filter(|(_, content)| content.contains(".same"))
        .collect::<Vec<_>>();
    let scripts = generated_assets(root, "js");
    assert_eq!(css.len(), 2);
    assert_eq!(scripts.len(), 2);
    for prefix in ["a.", "b."] {
        assert!(css.iter().any(|(name, _)| name.starts_with(prefix)));
        assert!(scripts.iter().any(|(name, _)| name.starts_with(prefix)));
    }
    assert_eq!(
        scripts
            .iter()
            .filter(|(_, content)| content.contains("sameComponent"))
            .count(),
        2
    );
    assert_eq!(
        css.iter()
            .filter(|(_, content)| content.contains("pixel."))
            .count(),
        2
    );
    assert!(
        css.iter()
            .all(|(_, content)| content.contains(".from-import"))
    );
    assert!(
        session
            .dependencies()
            .contains(&FilesystemDependency::File(imported.clone()))
    );
    for (name, _) in css.iter().chain(&scripts) {
        assert_eq!(html.matches(name).count(), 1, "{html}");
    }
    assert!(!html.contains(".same"), "{html}");
    assert!(!html.contains("sameComponent"), "{html}");
    let binary_assets = std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "bin"))
        .collect::<Vec<_>>();
    assert_eq!(binary_assets.len(), 1);
    let binary_name = binary_assets[0].file_name().to_string_lossy().into_owned();
    assert!(
        css.iter()
            .all(|(_, content)| content.contains(&binary_name))
    );
    assert!(css.iter().all(|(_, content)| !content.contains("_assets/")));

    std::fs::write(&imported, ".from-import { color: green; }").unwrap();
    session.build().unwrap();
    let rebuilt_css = generated_assets(root, "css")
        .into_iter()
        .filter(|(_, content)| content.contains(".same"))
        .collect::<Vec<_>>();
    assert_eq!(rebuilt_css.len(), 2);
    assert!(
        rebuilt_css
            .iter()
            .all(|(_, content)| content.contains("green") && !content.contains("blue"))
    );
}

#[test]
fn component_resources_preserve_document_order() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::write(
        root.join("components/outer.typ"),
        concat!(
            "#import \"./inner.typ\": inner\n",
            "#let outer() = [\n",
            "  #metadata(\"./outer-first.css\") <aster-style>\n",
            "  #inner()\n",
            "  #metadata(\"./outer-last.css\") <aster-style>\n",
            "]\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("components/inner.typ"),
        "#let inner() = [#metadata(\"./inner.css\") <aster-style>]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("components/outer-first.css"),
        ".outer-first { color: red; }",
    )
    .unwrap();
    std::fs::write(
        root.join("components/inner.css"),
        ".inner { color: green; }",
    )
    .unwrap();
    std::fs::write(
        root.join("components/outer-last.css"),
        ".outer-last { color: blue; }",
    )
    .unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#import \"/components/outer.typ\": outer\n",
            "#html.html({ html.head[]; html.body[#outer()] })\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let first = generated_asset_containing(root, "css", ".outer-first");
    let inner = generated_asset_containing(root, "css", ".inner");
    let last = generated_asset_containing(root, "css", ".outer-last");
    let first = html.find(&first).unwrap();
    let inner = html.find(&inner).unwrap();
    let last = html.find(&last).unwrap();
    assert!(first < inner && inner < last, "{html}");
}

#[test]
fn resource_content_rejects_multiple_raw_elements() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        r#"#metadata([
  ```css
  .first {}
  ```
  ```css
  .second {}
  ```
]) <aster-style>
#html.elem("p")[Page]
"#,
    )
    .unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("multiple raw elements must fail");
    assert!(
        format!("{error:#}").contains("exactly one raw element"),
        "{error:#}"
    );
}

fn generated_asset_containing(root: &std::path::Path, extension: &str, marker: &str) -> String {
    generated_assets(root, extension)
        .into_iter()
        .find_map(|(name, content)| content.contains(marker).then_some(name))
        .unwrap()
}

fn generated_assets(root: &std::path::Path, extension: &str) -> Vec<(String, String)> {
    std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|value| value == extension)
        })
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read_to_string(entry.path()).unwrap(),
            )
        })
        .collect()
}
