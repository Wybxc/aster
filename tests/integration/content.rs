use aster::build::world::TypstSession;
use aster::engine::content;
use aster::foundation::project::ProjectRoot;
use typst::foundations::{Str, Value};

#[test]
fn protocol_contains_lazy_entry_modules() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("content/blog/nested")).unwrap();
    std::fs::write(root.join("aster.toml"), "").unwrap();
    std::fs::write(root.join("content/blog/nested/post.typ"), "= Post").unwrap();
    let session = TypstSession::new(ProjectRoot::new(root.to_owned()).unwrap());

    let Value::Dict(protocol) = content::load(&session).unwrap() else {
        panic!("protocol must be a dictionary");
    };
    let Value::Dict(collections) = protocol.get("collections").unwrap() else {
        panic!("collections must be a dictionary");
    };
    let Value::Dict(blog) = collections.get("blog").unwrap() else {
        panic!("collection must be a dictionary");
    };
    let Value::Module(entry) = blog.get("nested/post").unwrap() else {
        panic!("entry must be a module");
    };
    assert_eq!(
        entry.field("id", ()).unwrap(),
        &Value::Str(Str::from("nested/post"))
    );
    assert_eq!(
        entry.field("collection", ()).unwrap(),
        &Value::Str(Str::from("blog"))
    );
    assert!(matches!(entry.field("render", ()).unwrap(), Value::Func(_)));
    assert!(entry.field("file-path", ()).is_err());
    assert!(entry.field("content", ()).is_err());
}
