use std::path::Path;

use anyhow::Result;
use aster::build::world::TypstSession;
use aster::engine::{content, route};
use aster::foundation::project::ProjectRoot;
use typst::foundations::Dict;

fn fixture(files: &[&str]) -> (tempfile::TempDir, ProjectRoot) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    for file in files {
        let path = root.join("src").join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    let project = ProjectRoot::new(root.to_owned()).unwrap();
    (temp, project)
}

fn build_plan(project: &ProjectRoot) -> Result<route::RoutePlan> {
    let session = TypstSession::new(project.clone());
    let inputs = content::install(Dict::new(), content::load(&session)?)?;
    let library = session.library(inputs.clone());
    route::RoutePlan::build(&session, &inputs, &library)
}

fn write_routes(project: &ProjectRoot, template: &str, routes: &str) {
    std::fs::write(
        project.src_dir().join(template),
        format!("#metadata({routes}) <route>"),
    )
    .unwrap();
}

#[test]
fn route_plan_is_sorted_and_probes_dynamic_templates() {
    let (_temp, project) = fixture(&["z.typ", "blog/[slug].typ", "a.typ"]);
    write_routes(&project, "blog/[slug].typ", "((slug: \"post\"),)");
    let plan = build_plan(&project).unwrap();
    let (jobs, warnings) = plan.into_parts();

    assert!(warnings.is_empty());
    assert_eq!(
        jobs.iter()
            .map(|job| job.output.as_path())
            .collect::<Vec<_>>(),
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
    assert!(build_plan(&project).is_err());
}

#[test]
fn route_plan_reports_missing_dynamic_metadata() {
    let (_temp, project) = fixture(&["[slug].typ"]);
    let plan = build_plan(&project).unwrap();
    let (jobs, warnings) = plan.into_parts();
    assert!(jobs.is_empty());
    assert_eq!(warnings.len(), 1);
}
