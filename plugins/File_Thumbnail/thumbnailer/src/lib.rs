//! # Thumbnailer
//!
//! This crate can be used to generate thumbnails for all kinds of files.
//!
//! Example:
//! ```
//! use thumbnailer::{create_thumbnails, Thumbnail, ThumbnailSize};
//! use std::fs::File;
//! use std::io::BufReader;
//! use std::io::Cursor;
//! use file_format::FileFormat;
//! let file = File::open("tests/assets/test.png").unwrap();
//! let reader = BufReader::new(file);
//! let mut  thumbnails = create_thumbnails(reader, FileFormat::PortableNetworkGraphics, [ThumbnailSize::Small, ThumbnailSize::Medium]).unwrap();
//!
//! let thumbnail = thumbnails.pop().unwrap();
//! let mut buf = Cursor::new(Vec::new());
//! thumbnail.write_png(&mut buf).unwrap();
//! ```
use crate::{error::ThumbResult, utils::ffmpeg_cli::get_webp_frame};
use file_format::FileFormat;
use image::{DynamicImage, GenericImageView, ImageFormat};
use rayon::prelude::*;
use std::io::{BufRead, BufReader, Seek, Write};

use crate::formats::get_base_image;
pub use size::ThumbnailSize;
use std::convert::From;

pub mod error;
mod formats;
mod size;
pub(crate) mod utils;

#[derive(Clone, Debug)]
pub struct Thumbnail {
    inner: DynamicImage,
    mime: FileFormat,
}

#[derive(Clone, Debug)]
pub enum FilterType {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl FilterType {
    const fn translate_filter(&self) -> image::imageops::FilterType {
        match self {
            Self::Nearest => image::imageops::FilterType::Nearest,
            Self::Triangle => image::imageops::FilterType::Triangle,
            Self::CatmullRom => image::imageops::FilterType::CatmullRom,
            Self::Gaussian => image::imageops::FilterType::Gaussian,
            Self::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

impl From<FilterType> for image::imageops::FilterType {
    fn from(filter_type: FilterType) -> Self {
        filter_type.translate_filter()
    }
}

impl Thumbnail {
    /// Writes the bytes of the image in a png format
    pub fn write_png<W: Write + Seek>(self, writer: &mut W) -> ThumbResult<()> {
        let image = DynamicImage::ImageRgba8(self.inner.into_rgba8());
        image.write_to(writer, ImageFormat::Png)?;

        Ok(())
    }

    /// Writes the bytes of the image in a jpeg format
    pub fn write_jpeg<W: Write + Seek>(self, writer: &mut W, quality: u8) -> ThumbResult<()> {
        let image = DynamicImage::ImageRgb8(self.inner.into_rgb8());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, quality);
        encoder.encode_image(&image)?;

        Ok(())
    }
    /// Writes the bytes of the image in a webp format
    #[cfg(feature = "webp")]
    pub fn write_webp<W: Write + Seek>(self, writer: &mut W) -> ThumbResult<()> {
        use image::EncodableLayout;
        use webp;
        let image = DynamicImage::ImageRgba8(self.inner.into_rgba8());
        let mut webp = webp::Encoder::from_image(&image).unwrap();
        let out = webp.encode(70.0);
        writer.write_all(out.as_bytes());
        Ok(())
    }

    /// Returns the fileformat that it's parsed
    pub fn return_fileformat(&self) -> FileFormat {
        self.mime
    }

    /// Returns the size of the thumbnail as width,  height
    pub fn size(&self) -> (u32, u32) {
        self.inner.dimensions()
    }
}

/// Creates thumbnails of the requested sizes for the given reader providing the content as bytes and
/// the mime describing the contents type
pub fn create_thumbnails_samplefilter<R: BufRead + Seek, I: IntoIterator<Item = ThumbnailSize>>(
    reader: R,
    mime: FileFormat,
    sizes: I,
    filter: FilterType,
) -> ThumbResult<Vec<Thumbnail>> {
    let image = get_base_image(reader, mime)?;
    let sizes: Vec<ThumbnailSize> = sizes.into_iter().collect();
    let thumbnails = resize_images(image, &sizes, filter)
        .into_iter()
        .map(|image| Thumbnail { inner: image, mime })
        .collect();

    Ok(thumbnails)
}

/// Creates thumbnails of the requested sizes for the given reader providing the content as bytes and
/// the mime describing the contents type
pub fn create_thumbnails<R: BufRead + Seek, I: IntoIterator<Item = ThumbnailSize>>(
    reader: R,
    mime: FileFormat,
    sizes: I,
) -> ThumbResult<Vec<Thumbnail>> {
    let image = get_base_image(reader, mime)?;
    let sizes: Vec<ThumbnailSize> = sizes.into_iter().collect();
    let thumbnails = resize_images(image, &sizes, FilterType::Lanczos3)
        .into_iter()
        .map(|image| Thumbnail { inner: image, mime })
        .collect();

    Ok(thumbnails)
}

///
/// Creates thumbnail of requestes size despite not knowing the mime.
///
pub fn create_thumbnails_unknown_type<R: BufRead + Seek, I: IntoIterator<Item = ThumbnailSize>>(
    reader: R,
    sizes: I,
) -> ThumbResult<Vec<Thumbnail>> {
    let mut temp = BufReader::new(reader);
    let mut temp1 = temp.fill_buf().unwrap();
    let le = temp1.len();
    let temp2 = &temp1[..];
    let mime = FileFormat::from_bytes(temp2);
    temp1.consume(le);

    let image = get_base_image(temp, mime)?;
    let sizes: Vec<ThumbnailSize> = sizes.into_iter().collect();
    let thumbnails = resize_images(image, &sizes, FilterType::Lanczos3)
        .into_iter()
        .map(|image| Thumbnail { inner: image, mime })
        .collect();

    Ok(thumbnails)
}

fn resize_images(
    image: DynamicImage,
    sizes: &[ThumbnailSize],
    filter_type: crate::FilterType,
) -> Vec<DynamicImage> {
    sizes
        .into_par_iter()
        .map(|size| {
            let (width, height) = size.dimensions();
            image.resize_exact(
                width,
                height,
                image::imageops::FilterType::from(filter_type.clone()),
            )
        })
        .collect()
}

///
/// Get's multiple frames if they exist
///
pub fn get_video_frame_multiple<R: BufRead + Seek>(
    mut reader: R,
    mime: FileFormat,
    ttl: usize,                // total number of frames to get
    split: usize,              // amount of frames inbetween to get
    scale: Option<(u32, u32)>, // Scales the image
) -> ThumbResult<Vec<DynamicImage>> {
    use crate::error::{self, ThumbError, ThumbResult};
    use crate::utils::ffmpeg_cli::{get_webp_frame, is_ffmpeg_installed};
    use image::io::Reader as ImageReader;
    use image::{DynamicImage, ImageFormat};
    use std::io::{BufRead, Cursor, ErrorKind, Seek};
    lazy_static::lazy_static! { static ref FFMPEG_INSTALLED: bool = is_ffmpeg_installed(); }
    if !*FFMPEG_INSTALLED {
        return Err(ThumbError::Unsupported(mime));
    }

    let tempdir = tempfile::tempdir()?;
    let path = std::path::PathBuf::from(tempdir.path())
        .join("video")
        .with_extension(mime.extension());

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    std::fs::write(&path, buf)?;

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    let mut frames = Vec::with_capacity(ttl);
    let ttlamt = match ttl {
        0 => 1,
        _ => ttl - 1,
    };
    for inve in 1..=ttlamt {
        let frame_to_get = inve * split;
        let png_bytes = match get_webp_frame(
            path.to_str()
                .expect("path to tmpdir contains invalid characters"),
            frame_to_get,
        ) {
            Err(_) => {
                return Ok(frames);
            }
            Ok(out) => out,
        }; // take the 16th frame
        let img = ImageReader::with_format(Cursor::new(png_bytes), ImageFormat::WebP).decode();
        match img {
            Ok(img) => {
                if let Some(size) = scale {
                    frames.push(
                        resize_images(img, &[ThumbnailSize::Custom(size)], FilterType::Lanczos3)[0]
                            .clone(),
                    );
                } else {
                    frames.push(img);
                }
            }
            Err(_) => break,
        }
    }
    tempdir.close()?;

    Ok(frames)
}
