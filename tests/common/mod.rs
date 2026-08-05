use std::path::{Path, PathBuf};

use aster::Project;

pub fn project(root: &Path) -> Project {
    let config = root.join("aster.toml");
    if !config.exists() {
        std::fs::write(
            config,
            "[site]\ntitle = \"Test Site\"\ndescription = \"\"\n",
        )
        .unwrap();
    }
    Project::open(root.to_owned()).unwrap()
}

pub fn install_library(root: &Path) {
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::create_dir_all(root.join("templates")).unwrap();
    std::fs::write(
        root.join("lib.typ"),
        include_str!("../../templates/default/lib.typ"),
    )
    .unwrap();
    std::fs::write(
        root.join("components/navigation.typ"),
        include_str!("../../templates/default/components/navigation.typ"),
    )
    .unwrap();
    std::fs::write(
        root.join("templates/site.typ"),
        include_str!("../../templates/default/templates/site.typ"),
    )
    .unwrap();
}

pub fn write_css_page(root: &Path) {
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[\n",
            "    #html.elem(\"link\", attrs: (\"rel\": \"stylesheet\", \"href\": \"/styles/style.css\"))\n",
            "    #html.elem(\"link\", attrs: (\"rel\": \"stylesheet\", \"href\": \"https://example.com/site.css\"))\n",
            "  ]\n",
            "  html.body[Page]\n",
            "})\n",
        ),
    )
    .unwrap();
}

pub fn generated_asset_containing(root: &Path, marker: &str) -> (PathBuf, String) {
    std::fs::read_dir(root.join("dist/_assets"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find_map(|path| {
            let content = std::fs::read_to_string(&path).ok()?;
            content.contains(marker).then_some((path, content))
        })
        .unwrap()
}
