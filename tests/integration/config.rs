use aster::{BuildSession, FilesystemDependency};

use crate::common::{install_content_adapter, project};

#[test]
fn build_honors_configured_layout_and_processing_options() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("entries/blog")).unwrap();
    std::fs::create_dir_all(root.join("fonts/nested")).unwrap();
    install_content_adapter(root);
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[paths]\n",
            "source = \"pages\"\n",
            "content = \"entries\"\n",
            "public = \"files\"\n",
            "output = \"public\"\n",
            "[output]\n",
            "assets = \"static/generated\"\n",
            "pretty = true\n",
            "[assets]\n",
            "image-inline-threshold = 3\n",
            "[css]\n",
            "minify = false\n",
            "[typst.fonts]\n",
            "paths = [\"fonts\"]\n",
            "system = false\n",
            "[highlight]\n",
            "enabled = false\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("entries/blog/post.typ"),
        "#metadata((title: \"Configured\",)) <frontmatter>\n\nEntry body",
    )
    .unwrap();
    std::fs::write(root.join("pages/style.css"), ".page { color: red; }").unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#import \"/lib/aster/content.typ\": get-entry\n",
            "#let post = get-entry(\"blog\", \"post\").render()\n",
            "#html.html({\n",
            "  html.head[\n",
            "    #html.elem(\"link\", attrs: (\"rel\": \"css\", \"href\": \"style.css\"))\n",
            "  ]\n",
            "  html.body[\n",
            "    #html.elem(\"p\", attrs: (class: \"page\"))[#post.metadata.title #post.content]\n",
            "    #html.elem(\"code\", attrs: (\"data-lang\": \"rust\"))[let x = 1;]\n",
            "    #html.elem(\"img\", attrs: (src: \"data:image/png;base64,AAAA\"))\n",
            "  ]\n",
            "})\n",
        ),
    )
    .unwrap();

    let project = project(root);
    let mut session = BuildSession::new(project);
    let outcome = session.build().unwrap();

    assert_eq!(outcome.outputs, [root.join("public/index.html")]);
    let html = std::fs::read_to_string(root.join("public/index.html")).unwrap();
    assert!(html.starts_with("<!DOCTYPE html>\n<html>"));
    assert!(html.contains("Configured"));
    assert!(html.contains("Entry body"));
    assert!(html.contains("static/generated/style."));
    assert!(html.contains("static/generated/img."));
    assert!(!html.contains("static/generated/highlight."));
    assert!(!html.contains("class=\"hl-"));

    let assets = std::fs::read_dir(root.join("public/static/generated"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(assets.len(), 2);
    let css = assets
        .iter()
        .find(|path| path.extension().is_some_and(|extension| extension == "css"))
        .map(std::fs::read_to_string)
        .unwrap()
        .unwrap();
    assert!(css.contains('\n'), "CSS should not be minified: {css}");

    let dependencies = session.dependencies();
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("pages"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("entries"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("fonts"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("files"))));
    assert!(
        !dependencies
            .iter()
            .any(|dependency| dependency.path().starts_with(root.join("public")))
    );
}

#[test]
fn rejects_output_that_overlaps_source_without_deleting_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    let source = root.join("src/index.typ");
    std::fs::write(&source, "#html.elem(\"p\")[Keep]").unwrap();
    std::fs::write(
        root.join("aster.toml"),
        "[paths]\noutput = \"src/generated\"\n",
    )
    .unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("overlapping output must fail");

    assert!(format!("{error:#}").contains("source and output directories must not overlap"));
    assert!(source.is_file());
}

#[test]
fn session_recovers_after_manifest_is_fixed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    let project = project(root);

    std::fs::write(root.join("aster.toml"), "[paths\n").unwrap();
    let mut session = BuildSession::new(project.clone());
    assert!(session.build().is_err());
    assert!(
        session
            .dependencies()
            .into_iter()
            .any(|dependency| dependency == FilesystemDependency::File(project.config_file()))
    );

    std::fs::remove_file(root.join("aster.toml")).unwrap();
    assert!(session.build().is_err());
    assert!(
        session
            .dependencies()
            .into_iter()
            .any(|dependency| dependency == FilesystemDependency::File(project.config_file()))
    );

    std::fs::write(root.join("aster.toml"), "").unwrap();
    session.build().unwrap();
    assert!(root.join("dist/index.html").is_file());
}
