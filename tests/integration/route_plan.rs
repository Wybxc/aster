use std::path::Path;

use aster::{BuildSession, Project};

fn fixture(files: &[&str]) -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    for file in files {
        let path = root.join("src").join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "#html.elem(\"p\")[Page]").unwrap();
    }
    let project = Project::open(root.to_owned()).unwrap();
    (temp, project)
}

fn write_routes(project: &Project, template: &str, routes: &str) {
    std::fs::write(
        project.src_dir().join(template),
        format!("#metadata({routes}) <route>\n#html.elem(\"p\")[Page]"),
    )
    .unwrap();
}

#[test]
fn route_plan_is_sorted_and_probes_dynamic_templates() {
    let (_temp, project) = fixture(&["z.typ", "blog/[slug].typ", "a.typ"]);
    write_routes(&project, "blog/[slug].typ", "((slug: \"post\"),)");

    let outcome = BuildSession::new(project.clone()).build().unwrap();
    let outputs = outcome
        .outputs
        .iter()
        .map(|path| path.strip_prefix(project.output_dir()).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        outputs,
        vec![
            Path::new("a.html"),
            Path::new("blog/post.html"),
            Path::new("z.html"),
        ]
    );
}

#[test]
fn route_plan_rejects_static_dynamic_collision() {
    let (_temp, project) = fixture(&["post.typ", "[slug].typ"]);
    write_routes(&project, "[slug].typ", "((slug: \"post\"),)");

    assert!(BuildSession::new(project).build().is_err());
}

#[test]
fn route_plan_reports_missing_dynamic_metadata() {
    let (_temp, project) = fixture(&["[slug].typ"]);

    let outcome = BuildSession::new(project).build().unwrap();

    assert!(outcome.outputs.is_empty());
    assert_eq!(outcome.warnings.len(), 1);
}
