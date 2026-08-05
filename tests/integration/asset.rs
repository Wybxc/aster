use std::path::{Path, PathBuf};

use aster::{BuildSession, FilesystemDependency};

use crate::common::project;

#[test]
fn publishes_html_resources_from_project_and_component_paths() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages/nested")).unwrap();
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(
        root.join("components/gallery.typ"),
        concat!(
            "#let gallery() = html.elem(\"picture\")[\n",
            "  #html.elem(\"source\", attrs: (srcset: \"./small.png 1x, /assets/large.png?v=2#hero 2x\"))\n",
            "  #html.elem(\"img\", attrs: (src: \"./small.png\", alt: \"Gallery\"))\n",
            "]\n",
        ),
    )
    .unwrap();
    std::fs::write(root.join("components/small.png"), b"small image").unwrap();
    std::fs::write(root.join("assets/large.png"), b"large image").unwrap();
    std::fs::write(root.join("assets/favicon.svg"), b"favicon").unwrap();
    std::fs::write(root.join("assets/social.png"), b"social image").unwrap();
    std::fs::write(root.join("assets/poster.jpg"), b"poster image").unwrap();
    std::fs::write(root.join("assets/guide.pdf"), b"guide").unwrap();
    std::fs::write(root.join("assets/site.webmanifest"), b"{}").unwrap();
    std::fs::write(
        root.join("pages/nested/index.typ"),
        concat!(
            "#import \"/components/gallery.typ\": gallery\n",
            "#html.html({\n",
            "  html.head[\n",
            "    #html.elem(\"link\", attrs: (rel: \"icon\", href: \"/assets/favicon.svg?rev=1#icon\"))\n",
            "    #html.elem(\"link\", attrs: (rel: \"manifest\", href: \"/assets/site.webmanifest\"))\n",
            "    #html.elem(\"meta\", attrs: (property: \"og:image\", content: \"/assets/social.png\"))\n",
            "  ]\n",
            "  html.body[\n",
            "    #gallery()\n",
            "    #html.elem(\"video\", attrs: (poster: \"/assets/poster.jpg\"))[]\n",
            "    #html.elem(\"a\", attrs: (href: \"/assets/guide.pdf\", download: \"\"))[Guide]\n",
            "    #html.elem(\"img\", attrs: (src: \"https://example.com/remote.png\", srcset: \"//cdn.example.com/remote.png 1x, /assets/large.png 2x\"))\n",
            "  ]\n",
            "})\n",
        ),
    )
    .unwrap();
    std::fs::write(root.join("aster.toml"), "[highlight]\nenabled = false\n").unwrap();

    let mut session = BuildSession::new(project(root));
    session.build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/nested/index.html")).unwrap();
    let small = generated_asset_with_content(root, b"small image");
    let large = generated_asset_with_content(root, b"large image");
    let favicon = generated_asset_with_content(root, b"favicon");
    let social = generated_asset_with_content(root, b"social image");
    let poster = generated_asset_with_content(root, b"poster image");
    let guide = generated_asset_with_content(root, b"guide");
    let manifest = generated_asset_with_content(root, b"{}");
    let url = |path: &Path| format!("../_assets/{}", path.file_name().unwrap().to_string_lossy());

    assert!(
        html.contains(&format!("{}?rev=1#icon", url(&favicon))),
        "{html}"
    );
    assert!(html.contains(&url(&manifest)), "{html}");
    assert!(html.contains(&url(&social)), "{html}");
    assert!(html.contains(&url(&poster)), "{html}");
    assert!(html.contains(&url(&guide)), "{html}");
    assert!(html.contains(&url(&small)), "{html}");
    assert!(
        html.contains(&format!("{}?v=2#hero 2x", url(&large))),
        "{html}"
    );
    assert!(html.contains("//cdn.example.com/remote.png 1x"), "{html}");
    assert!(
        html.contains("src=\"https://example.com/remote.png\""),
        "{html}"
    );
    for source in [
        root.join("components/small.png"),
        root.join("assets/large.png"),
        root.join("assets/favicon.svg"),
        root.join("assets/social.png"),
        root.join("assets/poster.jpg"),
        root.join("assets/guide.pdf"),
        root.join("assets/site.webmanifest"),
    ] {
        assert!(
            session
                .dependencies()
                .contains(&FilesystemDependency::File(source))
        );
    }
}

#[test]
fn rechecks_a_missing_html_resource() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        "#html.body[#html.elem(\"img\", attrs: (src: \"/assets/missing.svg\"))]",
    )
    .unwrap();
    let missing = root.join("assets/missing.svg");
    let mut session = BuildSession::new(project(root));

    assert!(session.build().is_err());
    assert!(
        session
            .dependencies()
            .contains(&FilesystemDependency::File(missing.clone()))
    );

    std::fs::create_dir_all(missing.parent().unwrap()).unwrap();
    std::fs::write(&missing, b"created").unwrap();
    session.build().unwrap();
    assert_eq!(
        std::fs::read(generated_asset_with_content(root, b"created")).unwrap(),
        b"created"
    );
}

fn generated_asset_with_content(root: &Path, expected: &[u8]) -> PathBuf {
    std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| std::fs::read(path).is_ok_and(|content| content == expected))
        .unwrap()
}
