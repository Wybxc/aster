use aster::BuildSession;

use crate::common::project;

#[test]
fn site_root_navigation_is_relative_to_each_output_page() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages/posts/hello")).unwrap();
    std::fs::create_dir_all(root.join("pages/docs")).unwrap();
    std::fs::create_dir_all(root.join("downloads")).unwrap();
    std::fs::write(root.join("pages/index.typ"), navigation_page()).unwrap();
    std::fs::write(root.join("pages/404.typ"), navigation_page()).unwrap();
    std::fs::write(root.join("pages/posts/hello/index.typ"), navigation_page()).unwrap();
    std::fs::write(root.join("pages/docs/about.typ"), navigation_page()).unwrap();
    std::fs::write(root.join("downloads/guide.pdf"), b"guide").unwrap();
    std::fs::write(root.join("aster.toml"), "[highlight]\nenabled = false\n").unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let index = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let file = std::fs::read_to_string(root.join("dist/404.html")).unwrap();
    let nested = std::fs::read_to_string(root.join("dist/posts/hello/index.html")).unwrap();
    let nested_file = std::fs::read_to_string(root.join("dist/docs/about.html")).unwrap();

    for html in [&index, &file] {
        assert_navigation(html, "./", "posts/", "map/", "search/");
    }
    assert_navigation(
        &nested,
        "../../",
        "../../posts/",
        "../../map/",
        "../../search/",
    );
    assert_navigation(&nested_file, "../", "../posts/", "../map/", "../search/");

    for html in [&index, &file, &nested, &nested_file] {
        for unchanged in [
            "../manual/",
            "#section",
            "?page=2",
            "https://example.com/about/",
            "//cdn.example.com/library.js",
        ] {
            assert!(html.contains(&format!("href=\"{unchanged}\"")), "{html}");
        }
        for rewritten in [
            "href=\"/\"",
            "href=\"/?view=all#top\"",
            "href=\"/posts/",
            "href=\"/map/",
            "action=\"/search/",
            "href=\"/downloads/",
        ] {
            assert!(!html.contains(rewritten), "{html}");
        }
    }

    assert_eq!(nested.matches("href=\"../../_assets/guide.").count(), 2);
}

fn navigation_page() -> &'static str {
    concat!(
        "#html.html({\n",
        "  html.head[]\n",
        "  html.body[\n",
        "    #html.elem(\"a\", attrs: (href: \"/\",))[Root]\n",
        "    #html.elem(\"a\", attrs: (href: \"/?view=all#top\",))[Root query]\n",
        "    #html.elem(\"a\", attrs: (href: \"/posts/?page=2#latest\",))[Posts]\n",
        "    #html.elem(\"area\", attrs: (href: \"/map/?view=all#top\", alt: \"Map\"))\n",
        "    #html.elem(\"area\", attrs: (href: \"/downloads/guide.pdf\", download: \"\", alt: \"Download\"))\n",
        "    #html.elem(\"form\", attrs: (action: \"/search/?q=aster\",))[]\n",
        "    #html.elem(\"a\", attrs: (href: \"../manual/\",))[Relative]\n",
        "    #html.elem(\"a\", attrs: (href: \"#section\",))[Fragment]\n",
        "    #html.elem(\"a\", attrs: (href: \"?page=2\",))[Query]\n",
        "    #html.elem(\"a\", attrs: (href: \"https://example.com/about/\",))[External]\n",
        "    #html.elem(\"a\", attrs: (href: \"//cdn.example.com/library.js\",))[CDN]\n",
        "    #html.elem(\"a\", attrs: (href: \"/downloads/guide.pdf\", download: \"\"))[Download]\n",
        "  ]\n",
        "})\n",
    )
}

fn assert_navigation(html: &str, root: &str, posts: &str, map: &str, search: &str) {
    assert!(html.contains(&format!("href=\"{root}\"")), "{html}");
    assert!(
        html.contains(&format!("href=\"{root}?view=all#top\"")),
        "{html}"
    );
    assert!(
        html.contains(&format!("href=\"{posts}?page=2#latest\"")),
        "{html}"
    );
    assert!(
        html.contains(&format!("href=\"{map}?view=all#top\"")),
        "{html}"
    );
    assert!(
        html.contains(&format!("action=\"{search}?q=aster\"")),
        "{html}"
    );
}
