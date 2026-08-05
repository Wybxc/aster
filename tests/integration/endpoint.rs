use aster::{BuildSession, FilesystemDependency};

use crate::common::project;

#[test]
fn build_publishes_endpoint_metadata_at_the_exact_template_path() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages/feed")).unwrap();
    std::fs::write(root.join("pages/index.typ"), "#html.elem(\"p\")[Page]").unwrap();
    std::fs::write(
        root.join("pages/feed/rss.xml.typ"),
        "#metadata(\"<?xml version=\\\"1.0\\\"?><rss/>\") <aster-endpoint>",
    )
    .unwrap();

    let outcome = BuildSession::new(project(root)).build().unwrap();

    assert_eq!(outcome.outputs, [root.join("dist/index.html")]);
    assert_eq!(outcome.endpoints, [root.join("dist/feed/rss.xml")]);
    assert_eq!(
        std::fs::read_to_string(root.join("dist/feed/rss.xml")).unwrap(),
        "<?xml version=\"1.0\"?><rss/>"
    );
    assert!(!root.join("dist/feed/rss.xml/index.html").exists());
}

#[test]
fn endpoint_bytes_are_tracked_and_refreshed_by_a_reused_session() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("pages")).unwrap();
    let payload = root.join("payload.bin");
    std::fs::write(&payload, [0, 1, 2]).unwrap();
    std::fs::write(
        root.join("pages/archive.bin.typ"),
        "#metadata(read(\"/payload.bin\", encoding: none)) <aster-endpoint>",
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
fn contextual_dynamic_endpoints_are_compiled_for_each_declared_route() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages/feed")).unwrap();
    std::fs::write(
        root.join("pages/feed/[slug].xml.typ"),
        concat!(
            "#let slug = sys.inputs.at(\"slug\", default: none)\n",
            "#context [\n",
            "  #metadata(((slug: \"alpha\"), (slug: \"beta\"))) <aster-route>\n",
            "  #metadata(if slug == none { none } else { \"<feed>\" + slug + \"</feed>\" }) <aster-endpoint>\n",
            "]\n",
        ),
    )
    .unwrap();

    let outcome = BuildSession::new(project(root)).build().unwrap();

    assert!(outcome.outputs.is_empty());
    assert_eq!(
        outcome.endpoints,
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
fn endpoint_metadata_requires_one_string_or_byte_payload() {
    for (source, expected) in [
        (
            "#metadata(42) <aster-endpoint>",
            "endpoint metadata must contain a string or bytes",
        ),
        (
            "#metadata(\"one\") <aster-endpoint>\n#metadata(\"two\") <aster-endpoint>",
            "endpoint template must contain exactly one <aster-endpoint> declaration",
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("pages")).unwrap();
        std::fs::write(root.join("pages/feed.xml.typ"), source).unwrap();

        let error = BuildSession::new(project(root))
            .build()
            .err()
            .expect("invalid endpoint metadata must fail");
        assert!(
            format!("{error:#}").contains(expected),
            "unexpected error: {error:#}"
        );
        assert!(!root.join("dist").exists());
    }
}
