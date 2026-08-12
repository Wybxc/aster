use aster::{BuildSession, FilesystemDependency};

use crate::common::{install_library, project};

#[test]
fn build_honors_configured_layout_and_processing_options() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("routes")).unwrap();
    std::fs::create_dir_all(root.join("artifacts")).unwrap();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    std::fs::create_dir_all(root.join("entries/blog")).unwrap();
    std::fs::create_dir_all(root.join("fonts/nested")).unwrap();
    install_library(root);
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[paths]\n",
            "pages = \"routes\"\n",
            "generate = \"artifacts\"\n",
            "content = \"entries\"\n",
            "public = \"files\"\n",
            "output = \"public\"\n",
            "[site]\n",
            "title = \"Test Site\"\n",
            "description = \"\"\n",
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
        "#metadata((title: \"Configured\",)) <aster-frontmatter>\n\nEntry body",
    )
    .unwrap();
    std::fs::write(root.join("styles/style.css"), ".page { color: red; }").unwrap();
    std::fs::write(
        root.join("routes/index.typ"),
        concat!(
            "#import \"/lib.typ\": get-entry\n",
            "#let settings = toml(\"/aster.toml\")\n",
            "#let post = get-entry(\"blog\", \"post\")\n",
            "#let metadata = post.metadata()\n",
            "#html.html({\n",
            "  html.head[\n",
            "    #html.elem(\"link\", attrs: (\"rel\": \"stylesheet\", \"href\": \"/styles/style.css\"))\n",
            "  ]\n",
            "  html.body[\n",
            "    #html.elem(\"p\")[#settings.site.title]\n",
            "    #html.elem(\"p\")[#sys.inputs.at(\"site\", default: \"not injected\")]\n",
            "    #html.elem(\"p\", attrs: (class: \"page\"))[#metadata.title #post.render()]\n",
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

    assert_eq!(outcome.pages, [root.join("public/index.html")]);
    let html = std::fs::read_to_string(root.join("public/index.html")).unwrap();
    assert!(html.starts_with("<!DOCTYPE html>\n<html>"));
    assert!(html.contains("Test Site"));
    assert!(html.contains("not injected"));
    assert!(html.contains("Configured"));
    assert!(html.contains("Entry body"));
    assert!(html.contains("static/generated/style."));
    assert!(!html.contains("static/generated/highlight."));
    assert!(!html.contains("class=\"hl-"));

    let assets = std::fs::read_dir(root.join("public/static/generated"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(assets.len(), 2);
    let image = assets
        .iter()
        .find(|path| path.extension().is_some_and(|extension| extension == "png"))
        .unwrap();
    let image_name = image.file_name().unwrap().to_string_lossy();
    let image_hash = image.file_stem().unwrap().to_string_lossy();
    assert_eq!(image_hash.len(), 16);
    assert!(image_hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(html.contains(&format!("static/generated/{image_name}")));
    let css = assets
        .iter()
        .find(|path| path.extension().is_some_and(|extension| extension == "css"))
        .map(std::fs::read_to_string)
        .unwrap()
        .unwrap();
    assert!(css.contains('\n'), "CSS should not be minified: {css}");

    let dependencies = session.dependencies();
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("routes"))));
    assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("artifacts"))));
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
fn rejects_output_that_overlaps_pages_without_deleting_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    let page = root.join("pages/index.typ");
    std::fs::write(&page, "#html.elem(\"p\")[Keep]").unwrap();
    std::fs::write(
        root.join("aster.toml"),
        "[paths]\noutput = \"pages/generated\"\n",
    )
    .unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("overlapping output must fail");

    assert!(format!("{error:#}").contains("pages and output directories must not overlap"));
    assert!(page.is_file());
}

#[test]
fn rejects_watch_path_overlapping_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    std::fs::write(
        root.join("aster.toml"),
        "[watch]\npaths = [\"dist/cache\"]\n",
    )
    .unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("overlapping watch path must fail");

    assert!(
        format!("{error:#}")
            .contains("watch path `dist/cache` must not overlap the output directory")
    );
}

#[test]
fn rejects_project_root_watch_path() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    std::fs::write(root.join("aster.toml"), "[watch]\npaths = [\".\"]\n").unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("project root watch path must fail");

    assert!(format!("{error:#}").contains("watch path cannot be the project root"));
}

#[test]
fn session_recovers_after_manifest_is_fixed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
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
