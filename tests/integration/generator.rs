use aster::{BuildSession, FilesystemDependency};

use crate::common::project;

#[test]
fn build_publishes_generator_output_at_the_exact_template_path() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("generate/feed")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    std::fs::write(
        root.join("generate/feed/rss.xml.typ"),
        "#metadata(\"<?xml version=\\\"1.0\\\"?><rss/>\") <aster-output>",
    )
    .unwrap();

    let outcome = BuildSession::new(project(root)).build().unwrap();

    assert_eq!(outcome.pages, [root.join("dist/index.html")]);
    assert_eq!(outcome.generated, [root.join("dist/feed/rss.xml")]);
    assert_eq!(
        std::fs::read_to_string(root.join("dist/feed/rss.xml")).unwrap(),
        "<?xml version=\"1.0\"?><rss/>"
    );
    assert!(!root.join("dist/feed/rss.xml/index.html").exists());
}

#[test]
fn generator_bytes_are_tracked_and_refreshed_by_a_reused_session() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::create_dir(root.join("generate")).unwrap();
    let payload = root.join("payload.bin");
    std::fs::write(&payload, [0, 1, 2]).unwrap();
    std::fs::write(
        root.join("generate/archive.bin.typ"),
        "#metadata(read(\"/payload.bin\", encoding: none)) <aster-output>",
    )
    .unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();
    assert_eq!(
        std::fs::read(root.join("dist/archive.bin")).unwrap(),
        [0, 1, 2]
    );
    assert!(
        session
            .dependencies()
            .contains(&FilesystemDependency::File(payload.clone()))
    );

    std::fs::write(&payload, [3, 4, 5, 6]).unwrap();
    session.build().unwrap();
    assert_eq!(
        std::fs::read(root.join("dist/archive.bin")).unwrap(),
        [3, 4, 5, 6]
    );
}

#[test]
fn contextual_dynamic_generators_are_compiled_for_each_declared_route() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("generate/feed")).unwrap();
    std::fs::write(
        root.join("generate/feed/[slug].xml.typ"),
        concat!(
            "#let slug = sys.inputs.at(\"_aster\").route.param(\"slug\")\n",
            "#context [\n",
            "  #metadata(((slug: \"alpha\"), (slug: \"beta\"))) <aster-route>\n",
            "  #metadata(if slug == none { none } else { \"<feed>\" + slug + \"</feed>\" }) <aster-output>\n",
            "]\n",
        ),
    )
    .unwrap();

    let outcome = BuildSession::new(project(root)).build().unwrap();

    assert!(outcome.pages.is_empty());
    assert_eq!(
        outcome.generated,
        [
            root.join("dist/feed/alpha.xml"),
            root.join("dist/feed/beta.xml"),
        ]
    );
    assert_eq!(
        std::fs::read_to_string(root.join("dist/feed/alpha.xml")).unwrap(),
        "<feed>alpha</feed>"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("dist/feed/beta.xml")).unwrap(),
        "<feed>beta</feed>"
    );
}

#[test]
fn generator_requires_one_string_or_byte_output() {
    for (source, expected) in [
        (
            "#metadata(42) <aster-output>",
            "generator output must be a string or bytes",
        ),
        (
            "#metadata(\"one\") <aster-output>\n#metadata(\"two\") <aster-output>",
            "generator must contain exactly one <aster-output> declaration",
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("pages")).unwrap();
        std::fs::create_dir(root.join("generate")).unwrap();
        std::fs::write(root.join("generate/feed.xml.typ"), source).unwrap();

        let error = BuildSession::new(project(root))
            .build()
            .err()
            .expect("invalid generator output must fail");
        assert!(
            format!("{error:#}").contains(expected),
            "unexpected error: {error:#}"
        );
        assert!(!root.join("dist").exists());
    }
}

#[test]
fn generator_receives_final_labelled_page_content() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::create_dir(root.join("generate")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#html.article[Hello #html.strong[world]] <aster-content>]\n",
            "})\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("generate/snapshot.txt.typ"),
        concat!(
            "#let page = sys.inputs.at(\"_aster\").site.pages.first()\n",
            "#let body = page.content\n",
            "#metadata(page.path + \"\\n\" + body.html + \"\\n\" + body.text) <aster-output>\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let snapshot = std::fs::read_to_string(root.join("dist/snapshot.txt")).unwrap();
    assert!(snapshot.starts_with("/\n<article>"), "{snapshot}");
    assert!(snapshot.contains("<strong>world</strong>"), "{snapshot}");
    assert!(snapshot.ends_with("\nHello world"), "{snapshot}");
}

#[test]
fn content_marker_survives_head_insertion_during_transforms() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::create_dir(root.join("generate")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html(html.body[\n",
            "  #html.article[```rust\nlet value = 1;\n```] <aster-content>\n",
            "])\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("generate/snapshot.html.typ"),
        concat!(
            "#let page = sys.inputs._aster.site.pages.first()\n",
            "#metadata(page.content.html) <aster-output>\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let page = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let snapshot = std::fs::read_to_string(root.join("dist/snapshot.html")).unwrap();
    assert!(page.contains("<head>"), "{page}");
    assert!(snapshot.starts_with("<article>"), "{snapshot}");
    assert!(snapshot.contains("class=\"hl-"), "{snapshot}");
    assert!(!page.contains("data-aster-content-root"), "{page}");
    assert!(!snapshot.contains("data-aster-content-root"), "{snapshot}");
}

#[test]
fn page_rejects_multiple_content_roots() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html(html.body[\n",
            "  #html.article[First] <aster-content>\n",
            "  #html.article[Second] <aster-content>\n",
            "])\n",
        ),
    )
    .unwrap();

    let error = BuildSession::new(project(root))
        .build()
        .err()
        .expect("multiple content roots must fail");

    assert!(
        format!("{error:#}").contains("at most one <aster-content> element"),
        "unexpected error: {error:#}"
    );
}
