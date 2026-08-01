use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aster::build::output::{OutputPath, OutputPublication};
use aster::foundation::project::ProjectRoot;

fn fixture() -> (tempfile::TempDir, ProjectRoot) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src/blog")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    let project = ProjectRoot::new(root.to_owned()).unwrap();
    (temp, project)
}

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut snapshot = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let relative = entry.path().strip_prefix(root).unwrap().to_owned();
            snapshot.insert(relative, std::fs::read(entry.path()).unwrap());
        }
    }
    snapshot
}

#[test]
fn source_resolution_uses_template_directory_and_is_confined() {
    let (_temp, project) = fixture();
    std::fs::write(project.src_dir().join("style.css"), "body{}").unwrap();
    let template = project.src_dir().join("blog/[slug].typ");
    std::fs::write(&template, "").unwrap();
    let mut publication = OutputPublication::new(&project);
    let output = OutputPath::new("blog/post.html").unwrap();
    let page = publication.page(&template, &output).unwrap();

    assert_eq!(
        page.resolve_source(Path::new("../style.css")).unwrap(),
        std::fs::canonicalize(project.src_dir().join("style.css")).unwrap()
    );
    assert!(page.resolve_source(Path::new("../../aster.toml")).is_err());
}

#[test]
fn publication_is_idempotent_and_removes_stale_output() {
    let (temp, project) = fixture();
    std::fs::create_dir_all(project.output_dir()).unwrap();
    std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

    let mut publication = OutputPublication::new(&project);
    let first = publication
        .add_asset("css", "css", b"body{}".to_vec())
        .unwrap();
    let second = publication
        .add_asset("css", "css", b"body{}".to_vec())
        .unwrap();
    assert_eq!(first, second);

    let template = project.src_dir().join("index.typ");
    std::fs::write(&template, "").unwrap();
    let output = OutputPath::new("index.html").unwrap();
    publication
        .page(&template, &output)
        .unwrap()
        .add_html("new".into())
        .unwrap();
    publication.publish().unwrap();

    let expected = snapshot_tree(&project.output_dir());
    std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

    let mut repeated = OutputPublication::new(&project);
    assert_eq!(
        repeated
            .add_asset("css", "css", b"body{}".to_vec())
            .unwrap(),
        first
    );
    repeated
        .page(&template, &output)
        .unwrap()
        .add_html("new".into())
        .unwrap();
    repeated.publish().unwrap();

    assert_eq!(snapshot_tree(&project.output_dir()), expected);
    assert!(!temp.path().join(".dist.aster-lock").exists());
    assert_eq!(expected.len(), 2);
}

#[test]
fn empty_publication_replaces_output_with_empty_directory() {
    let (_temp, project) = fixture();
    std::fs::create_dir_all(project.output_dir()).unwrap();
    std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

    OutputPublication::new(&project).publish().unwrap();

    assert!(project.output_dir().is_dir());
    assert_eq!(std::fs::read_dir(project.output_dir()).unwrap().count(), 0);
}
