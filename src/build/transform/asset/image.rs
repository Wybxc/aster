use std::io::Cursor;

use anyhow::{Result, ensure};
use image::codecs::jpeg::{JpegDecoder, JpegEncoder};
use image::codecs::png::{PngDecoder, PngEncoder};
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageEncoder, ImageFormat};
use typst::ecow::EcoString;
use typst::foundations::Bytes;
use typst::layout::{Frame, FrameItem, Size, Transform};
use typst::visualize::{ExchangeFormat, Image, RasterImage};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};
use typst_svg::WebImage;

use crate::foundation::config::ImageConfig;

use super::super::dom::HtmlElementExt;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(super) struct ImageSizeLimit {
    width: Option<u32>,
    height: Option<u32>,
}

impl ImageSizeLimit {
    pub fn from_element(element: &HtmlElement) -> Self {
        Self {
            width: image_dimension(element.get_attr("width").as_deref()),
            height: image_dimension(element.get_attr("height").as_deref()),
        }
    }

    pub fn for_candidate(
        self,
        width: Option<u64>,
        height: Option<u64>,
        density: Option<f64>,
    ) -> Self {
        let width = width
            .and_then(|value| u32::try_from(value).ok())
            .or_else(|| scaled_dimension(self.width, density));
        let height = height
            .and_then(|value| u32::try_from(value).ok())
            .or_else(|| scaled_dimension(self.height, density));
        Self { width, height }
    }
}

fn image_dimension(value: Option<&str>) -> Option<u32> {
    value?.parse().ok().filter(|value| *value > 0)
}

fn scaled_dimension(value: Option<u32>, density: Option<f64>) -> Option<u32> {
    let value = f64::from(value?);
    let density = density.filter(|density| density.is_finite() && *density > 0.0)?;
    let scaled = (value * density).ceil();
    (scaled <= f64::from(u32::MAX)).then_some(scaled as u32)
}

/// Applies one image policy to HTML, CSS, data URLs, and Typst frames.
#[derive(Clone)]
pub(super) struct ImageProcessor {
    config: ImageConfig,
}

impl ImageProcessor {
    pub fn new(config: &ImageConfig) -> Result<Self> {
        ensure!(
            (1..=100).contains(&config.jpeg_quality),
            "assets.images.jpeg-quality must be between 1 and 100"
        );
        ensure!(
            config.frame_density.is_none_or(|density| density > 0),
            "assets.images.frame-density must be greater than zero"
        );
        Ok(Self {
            config: config.clone(),
        })
    }

    pub fn process(&self, content: Bytes, limit: ImageSizeLimit) -> Bytes {
        optimize_image(content, limit, &self.config)
    }

    pub fn process_frames(&self, document: &mut HtmlDocument) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        process_frame_elements(document.root_mut(), self)
    }
}

fn process_frame_elements(element: &mut HtmlElement, processor: &ImageProcessor) -> Result<()> {
    for node in element.children.make_mut() {
        match node {
            HtmlNode::Element(element) => {
                process_frame_elements(element, processor)?;
            }
            HtmlNode::Frame(frame) => {
                process_frame(
                    &mut frame.inner,
                    Transform::identity(),
                    processor.config.frame_density,
                    processor,
                )?;
            }
            HtmlNode::Tag(_) | HtmlNode::Text(..) => {}
        }
    }
    Ok(())
}

fn process_frame(
    frame: &mut Frame,
    transform: Transform,
    density: Option<u8>,
    processor: &ImageProcessor,
) -> Result<()> {
    let mut error = None;
    frame.retain(|item| {
        if error.is_none() {
            let result = match item {
                FrameItem::Group(group) => process_frame(
                    &mut group.frame,
                    transform.pre_concat(group.transform),
                    density,
                    processor,
                ),
                FrameItem::Image(image, size, _) => {
                    let limit = density
                        .map(|density| frame_image_size_limit(*size, transform, density))
                        .unwrap_or_default();
                    process_frame_image(image, limit, processor)
                }
                FrameItem::Text(_)
                | FrameItem::Shape(..)
                | FrameItem::Link(..)
                | FrameItem::Tag(_) => Ok(()),
            };
            if let Err(cause) = result {
                error = Some(cause);
            }
        }
        true
    });
    match error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn process_frame_image(
    image: &mut Image,
    limit: ImageSizeLimit,
    processor: &ImageProcessor,
) -> Result<()> {
    let web_image = WebImage::new(image);
    let content = processor.process(web_image.data.clone(), limit);
    if content == web_image.data {
        return Ok(());
    }

    let Some(format) = ExchangeFormat::detect(content.as_slice()) else {
        return Ok(());
    };
    let raster = RasterImage::plain(content, format)
        .map_err(|error| anyhow::anyhow!("optimized Typst frame image is invalid: {error}"))?;
    *image = Image::new(raster, image.alt().map(EcoString::from), image.scaling());
    Ok(())
}

fn frame_image_size_limit(size: Size, transform: Transform, density: u8) -> ImageSizeLimit {
    let width = transform.sx.get().hypot(transform.ky.get()) * size.x.to_pt();
    let height = transform.kx.get().hypot(transform.sy.get()) * size.y.to_pt();
    let scale = (96.0 / 72.0) * f64::from(density);
    ImageSizeLimit {
        width: pixel_dimension(width * scale),
        height: pixel_dimension(height * scale),
    }
}

fn pixel_dimension(value: f64) -> Option<u32> {
    let value = value.ceil();
    (value.is_finite() && value > 0.0 && value <= f64::from(u32::MAX)).then_some(value as u32)
}

#[comemo::memoize]
fn optimize_image(content: Bytes, limit: ImageSizeLimit, config: &ImageConfig) -> Bytes {
    if !config.enabled {
        return content;
    }
    let Ok(format) = image::guess_format(content.as_slice()) else {
        return content;
    };
    match format {
        ImageFormat::Png => optimize_png(content, limit),
        ImageFormat::Jpeg => optimize_jpeg(content, limit, config.jpeg_quality),
        ImageFormat::Gif | ImageFormat::WebP => content,
        _ => content,
    }
}

fn optimize_png(content: Bytes, limit: ImageSizeLimit) -> Bytes {
    if limit != ImageSizeLimit::default()
        && let Some(resized) = try_resize_png(content.as_slice(), limit)
        && resized.len() < content.len()
    {
        return Bytes::new(resized);
    }
    smaller_png(content.clone()).unwrap_or(content)
}

fn optimize_jpeg(content: Bytes, limit: ImageSizeLimit, quality: u8) -> Bytes {
    let Some(encoded) = try_encode_jpeg(content.as_slice(), limit, quality) else {
        return content;
    };
    if encoded.len() < content.len() {
        Bytes::new(encoded)
    } else {
        content
    }
}

fn try_resize_png(content: &[u8], limit: ImageSizeLimit) -> Option<Vec<u8>> {
    let decoder = PngDecoder::with_limits(Cursor::new(content), image::Limits::default()).ok()?;
    if decoder.is_apng().ok()? {
        return None;
    }
    let (mut image, icc_profile) = decode_image(decoder)?;
    resize_image(&mut image, limit)?;

    let mut encoded = Vec::new();
    let mut encoder = PngEncoder::new(&mut encoded);
    if let Some(profile) = icc_profile {
        encoder.set_icc_profile(profile).ok();
    }
    image.write_with_encoder(encoder).ok()?;
    let encoded = Bytes::new(encoded);
    Some(smaller_png(encoded.clone()).unwrap_or(encoded).into_vec())
}

fn try_encode_jpeg(content: &[u8], limit: ImageSizeLimit, quality: u8) -> Option<Vec<u8>> {
    let decoder = JpegDecoder::new(Cursor::new(content)).ok()?;
    let (mut image, icc_profile) = decode_image(decoder)?;
    resize_image(&mut image, limit);

    let mut encoded = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut encoded, quality);
    if let Some(profile) = icc_profile {
        encoder.set_icc_profile(profile).ok();
    }
    image.write_with_encoder(encoder).ok()?;
    Some(encoded)
}

fn decode_image(mut decoder: impl ImageDecoder) -> Option<(DynamicImage, Option<Vec<u8>>)> {
    decoder.set_limits(image::Limits::default()).ok()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let icc_profile = decoder.icc_profile().ok().flatten();
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    Some((image, icc_profile))
}

fn resize_image(image: &mut DynamicImage, limit: ImageSizeLimit) -> Option<()> {
    if let Some((width, height)) = target_dimensions(image.dimensions(), limit) {
        *image = image.resize_exact(width, height, FilterType::Lanczos3);
        Some(())
    } else {
        None
    }
}

fn target_dimensions((width, height): (u32, u32), limit: ImageSizeLimit) -> Option<(u32, u32)> {
    let requested_width = limit.width.unwrap_or(width);
    let requested_height = limit.height.unwrap_or(height);
    if requested_width >= width && requested_height >= height {
        return None;
    }

    let (target_width, target_height) = if u64::from(requested_width) * u64::from(height)
        <= u64::from(requested_height) * u64::from(width)
    {
        (requested_width, scaled_axis(height, requested_width, width))
    } else {
        (
            scaled_axis(width, requested_height, height),
            requested_height,
        )
    };
    Some((target_width.max(1), target_height.max(1)))
}

fn scaled_axis(axis: u32, target: u32, source: u32) -> u32 {
    ((u64::from(axis) * u64::from(target) + u64::from(source) / 2) / u64::from(source))
        .try_into()
        .unwrap_or(u32::MAX)
}

fn smaller_png(content: Bytes) -> Option<Bytes> {
    let mut options = oxipng::Options::from_preset(2);
    options.max_decompressed_size = Some(512 * 1024 * 1024);
    let optimized = oxipng::optimize_from_memory(content.as_slice(), &options).ok()?;
    (optimized.len() < content.len()).then(|| Bytes::new(optimized))
}

#[cfg(test)]
mod tests {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ImageBuffer, Rgb};

    use super::*;

    fn png(width: u32, height: u32) -> Bytes {
        let image = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8])
        });
        let mut content = Vec::new();
        image
            .write_with_encoder(PngEncoder::new(&mut content))
            .unwrap();
        Bytes::new(content)
    }

    fn jpeg(width: u32, height: u32) -> Bytes {
        let image = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8])
        });
        let mut content = Vec::new();
        image
            .write_with_encoder(JpegEncoder::new_with_quality(&mut content, 95))
            .unwrap();
        Bytes::new(content)
    }

    #[test]
    fn downsamples_without_changing_aspect_ratio() {
        let content = png(80, 40);
        let processed = optimize_image(
            content,
            ImageSizeLimit {
                width: Some(20),
                height: Some(20),
            },
            &ImageConfig::default(),
        );

        let image = image::load_from_memory(processed.as_slice()).unwrap();
        assert_eq!(image.dimensions(), (20, 10));
    }

    #[test]
    fn never_upscales_images() {
        let content = png(8, 4);
        let processed = optimize_image(
            content.clone(),
            ImageSizeLimit {
                width: Some(80),
                height: Some(40),
            },
            &ImageConfig::default(),
        );

        let image = image::load_from_memory(processed.as_slice()).unwrap();
        assert_eq!(image.dimensions(), (8, 4));
        assert!(processed.len() <= content.len());
    }

    #[test]
    fn resizes_jpeg_with_the_configured_encoder() {
        let processed = optimize_image(
            jpeg(80, 40),
            ImageSizeLimit {
                width: Some(20),
                height: Some(20),
            },
            &ImageConfig::default(),
        );

        let image = image::load_from_memory(processed.as_slice()).unwrap();
        assert_eq!(image.dimensions(), (20, 10));
    }

    #[test]
    fn optimizes_jpeg_without_a_size_request() {
        let content = jpeg(80, 40);
        let processed = optimize_image(
            content.clone(),
            ImageSizeLimit::default(),
            &ImageConfig::default(),
        );

        assert!(processed.len() < content.len());
        let image = image::load_from_memory(processed.as_slice()).unwrap();
        assert_eq!(image.dimensions(), (80, 40));
    }

    #[test]
    fn leaves_animated_formats_untouched() {
        let content = Bytes::new(b"GIF89a not decoded by the optimizer");
        assert_eq!(
            optimize_image(
                content.clone(),
                ImageSizeLimit {
                    width: Some(1),
                    height: Some(1),
                },
                &ImageConfig::default(),
            ),
            content
        );
    }
}
