use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{GenericImageView, ImageBuffer, Rgb};

use aster::BuildSession;

use crate::common::project;

#[test]
fn downsamples_project_images_to_declared_html_dimensions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    write_png(&root.join("assets/photo.png"), 120, 60);
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#html.img(src: \"/assets/photo.png\", width: 30, height: 30, alt: \"\")]\n",
            "})\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(html.contains("src=\"_assets/photo."), "{html}");
    let image = only_generated_png(root);
    let image = image::open(image).unwrap();
    assert_eq!(image.dimensions(), (30, 15));
}

#[test]
fn optimizes_images_inside_inline_typst_frames() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    write_png(&root.join("assets/frame.png"), 300, 150);
    std::fs::write(
        root.join("aster.toml"),
        concat!(
            "[site]\n",
            "title = \"Frame test\"\n",
            "[assets.images]\n",
            "frame-density = 1\n",
            "[output]\n",
            "pretty = true\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#html.frame[#stack(\n",
            "    image(\"/assets/frame.png\", width: 75pt),\n",
            "    image(\"/assets/frame.png\", width: 30pt),\n",
            "  )]]\n",
            "})\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(html.contains("<svg"), "{html}");
    assert!(!html.contains("xlink:href=\"_assets/"), "{html}");
    let mut dimensions = embedded_pngs(&html)
        .iter()
        .map(|content| image::load_from_memory(content).unwrap().dimensions())
        .collect::<Vec<_>>();
    dimensions.sort_unstable();
    assert_eq!(dimensions, [(40, 20), (100, 50)]);
    assert!(generated_pngs(root).is_empty());
}

#[test]
fn keeps_frame_images_at_source_size_without_a_density_limit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    write_png(&root.join("assets/frame.png"), 120, 60);
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[]\n",
            "  html.body[#html.frame[#image(\"/assets/frame.png\", width: 30pt)]]\n",
            "})\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    let images = embedded_pngs(&html);
    assert_eq!(images.len(), 1, "{html}");
    assert_eq!(
        image::load_from_memory(&images[0]).unwrap().dimensions(),
        (120, 60)
    );
    assert!(generated_pngs(root).is_empty());
}

#[test]
fn optimizes_images_discovered_in_css() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    let source = png_bytes(120, 60, true);
    std::fs::write(root.join("assets/background.png"), &source).unwrap();
    std::fs::write(
        root.join("styles/site.css"),
        "body { background-image: url('/assets/background.png'); }",
    )
    .unwrap();
    std::fs::write(
        root.join("pages/index.typ"),
        concat!(
            "#html.html({\n",
            "  html.head[#html.link(rel: \"stylesheet\", href: \"/styles/site.css\")]\n",
            "  html.body[Page]\n",
            "})\n",
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let optimized = std::fs::read(only_generated_png(root)).unwrap();
    assert!(optimized.len() < source.len());
    assert_eq!(
        image::load_from_memory(&optimized).unwrap().dimensions(),
        (120, 60)
    );
}

#[test]
fn extracts_and_downsamples_image_data_urls() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pages")).unwrap();
    std::fs::write(
        root.join("aster.toml"),
        "[site]\ntitle = \"Data URL test\"\n[assets]\nimage-inline-threshold = 0\n",
    )
    .unwrap();
    let data = png_bytes(120, 60, false);
    let encoded = data
        .iter()
        .map(|byte| format!("%{byte:02X}"))
        .collect::<String>();
    let data_url = format!("data:image/png,{encoded}");
    std::fs::write(
        root.join("pages/index.typ"),
        format!(
            "#html.html({{ html.head[]; html.body[#html.img(src: {data_url:?}, width: 30, height: 30, alt: \"\")] }})\n"
        ),
    )
    .unwrap();

    BuildSession::new(project(root)).build().unwrap();

    let html = std::fs::read_to_string(root.join("dist/index.html")).unwrap();
    assert!(!html.contains("data:image/png"), "{html}");
    let image = image::open(only_generated_png(root)).unwrap();
    assert_eq!(image.dimensions(), (30, 15));
}

fn write_png(path: &std::path::Path, width: u32, height: u32) {
    std::fs::write(path, png_bytes(width, height, false)).unwrap();
}

fn png_bytes(width: u32, height: u32, fast: bool) -> Vec<u8> {
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8])
    });
    let mut content = Vec::new();
    if fast {
        image
            .write_with_encoder(PngEncoder::new_with_quality(
                &mut content,
                CompressionType::Fast,
                FilterType::NoFilter,
            ))
            .unwrap();
    } else {
        image
            .write_with_encoder(PngEncoder::new(&mut content))
            .unwrap();
    }
    content
}

fn only_generated_png(root: &std::path::Path) -> std::path::PathBuf {
    let images = generated_pngs(root);
    assert_eq!(images.len(), 1, "{images:?}");
    images.into_iter().next().unwrap()
}

fn generated_pngs(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let directory = root.join("dist/_assets");
    if !directory.exists() {
        return Vec::new();
    }
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "png"))
        .collect()
}

fn embedded_pngs(html: &str) -> Vec<Vec<u8>> {
    const PREFIX: &str = "data:image/png;base64,";

    let mut images = Vec::new();
    let mut remaining = html;
    while let Some(offset) = remaining.find(PREFIX) {
        let url = &remaining[offset..];
        let end = url
            .find('"')
            .expect("embedded image URL must end in an attribute");
        let data_url = data_url::DataUrl::process(&url[..end]).unwrap();
        images.push(data_url.decode_to_vec().unwrap().0);
        remaining = &url[end..];
    }
    images
}
