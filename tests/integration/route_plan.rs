use std::path::{Path, PathBuf};

use aster::{BuildSession, Project};
use typst::syntax::VirtualPath;

fn fixture(files: &[&str]) -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    for file in files {
        let path = root.join("pages").join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "#html.elem(\"p\")[Page]").unwrap();
    }
    let project = Project::open(root.to_owned()).unwrap();
    (temp, project)
}

fn write_routes(project: &Project, template: &str, routes: &str) {
    std::fs::write(
        project.root().join("pages").join(template),
        format!("#metadata({routes}) <route>\n#html.elem(\"p\")[Page]"),
    )
    .unwrap();
}

#[test]
fn route_plan_is_sorted_and_probes_dynamic_templates() {
    let (_temp, project) = fixture(&["z/index.typ", "blog/[slug]/index.typ", "a/index.typ"]);
    write_routes(&project, "blog/[slug]/index.typ", "((slug: \"post\"),)");

    let outcome = BuildSession::new(project.clone()).build().unwrap();
    let output_dir = project.root().join("dist");
    let outputs = outcome
        .outputs
        .iter()
        .map(|path| {
            let path = VirtualPath::virtualize(&output_dir, path).unwrap();
            PathBuf::from(path.get_without_slash())
        })
        .collect::<Vec<_>>();

    assert_eq!(
        outputs,
        vec![
            Path::new("a/index.html"),
            Path::new("blog/post/index.html"),
            Path::new("z/index.html"),
        ]
    );
}

#[test]
fn route_plan_preserves_file_shaped_page_routes() {
    let (_temp, project) = fixture(&["a.typ", "blog/[slug].typ"]);
    write_routes(&project, "blog/[slug].typ", "((slug: \"post\"),)");

    let outcome = BuildSession::new(project.clone()).build().unwrap();
    let output_dir = project.root().join("dist");
    let outputs = outcome
        .outputs
        .iter()
        .map(|path| {
            let path = VirtualPath::virtualize(&output_dir, path).unwrap();
            PathBuf::from(path.get_without_slash())
        })
        .collect::<Vec<_>>();

    assert_eq!(
        outputs,
        vec![Path::new("a.html"), Path::new("blog/post.html")]
    );
}

#[test]
fn route_plan_rejects_static_dynamic_collision() {
    let (_temp, project) = fixture(&["post.typ", "[slug].typ"]);
    write_routes(&project, "[slug].typ", "((slug: \"post\"),)");

    assert!(BuildSession::new(project).build().is_err());
}

#[test]
fn route_plan_rejects_page_endpoint_collision() {
    let (_temp, project) = fixture(&["feed.typ", "feed.html.typ"]);
    std::fs::write(
        project.root().join("pages/feed.html.typ"),
        "#metadata(\"feed\") <endpoint>",
    )
    .unwrap();

    let error = BuildSession::new(project)
        .build()
        .err()
        .expect("page and endpoint collision must fail");
    assert!(format!("{error:#}").contains("conflicting outputs feed.html and feed.html"));
}

#[test]
fn route_plan_reports_missing_dynamic_metadata() {
    let (_temp, project) = fixture(&["[slug].typ"]);

    let outcome = BuildSession::new(project).build().unwrap();

    assert!(outcome.outputs.is_empty());
    assert!(
        outcome
            .warnings
            .iter()
            .any(|warning| warning.as_str().contains("no <route> metadata"))
    );
}
