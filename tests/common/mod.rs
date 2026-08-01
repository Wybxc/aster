use std::path::{Path, PathBuf};

use aster::build::pipeline::BuildDriver;
use aster::foundation::config::AsterConfig;
use aster::foundation::project::ProjectRoot;

pub fn project(root: &Path) -> ProjectRoot {
    let config = root.join("aster.toml");
    if !config.exists() {
        std::fs::write(config, "").unwrap();
    }
    ProjectRoot::new(root.to_owned()).unwrap()
}

pub fn build(driver: &mut BuildDriver, project: &ProjectRoot) {
    driver
        .build(AsterConfig::load(&project.config_file()).unwrap())
        .unwrap();
}

pub fn install_content_adapter(root: &Path) {
    std::fs::create_dir_all(root.join("lib/aster")).unwrap();
    std::fs::write(
        root.join("lib/aster/content.typ"),
        include_str!("../../templates/default/lib/aster/content.typ"),
    )
    .unwrap();
}

pub fn write_css_page(root: &Path) {
    std::fs::write(
        root.join("src/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[\n",
            "    #html.elem(\"link\", attrs: (\"rel\": \"css\", \"href\": \"style.css\"))\n",
            "  ]\n",
            "  html.body[Page]\n",
            "})\n",
        ),
    )
    .unwrap();
}

pub fn generated_asset(project: &ProjectRoot, prefix: &str) -> (PathBuf, String) {
    let path = std::fs::read_dir(project.output_dir().join("_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    (path, content)
}
