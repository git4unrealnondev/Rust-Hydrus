const PNG_BYTES: &[u8] = include_bytes!("assets/test.png");
const JPG_BYTES: &[u8] = include_bytes!("assets/test.jpg");
const WEBP_BYTES: &[u8] = include_bytes!("assets/test.webp");

use crate::ImageType::{Jpeg, Png, Webp};
use file_format::FileFormat;
use std::io::Cursor;
use std::str::FromStr;
use thumbnailer::error::ThumbResult;
use thumbnailer::{create_thumbnails, create_thumbnails_unknown_type, Thumbnail, ThumbnailSize};

enum ImageType {
    Png,
    Jpeg,
    Webp,
}

#[test]
fn it_creates_small_thumbnails_for_png() {
    create_thumbnail(Png, ThumbnailSize::Small).unwrap();
}

#[test]
fn it_creates_medium_thumbnails_for_png() {
    create_thumbnail(Png, ThumbnailSize::Medium).unwrap();
}

#[test]
fn it_creates_large_thumbnails_for_png() {
    create_thumbnail(Png, ThumbnailSize::Large).unwrap();
}

#[test]
fn it_creates_small_thumbnails_for_jpeg() {
    create_thumbnail(Jpeg, ThumbnailSize::Small).unwrap();
}

#[test]
fn it_creates_medium_thumbnails_for_jpeg() {
    create_thumbnail(Jpeg, ThumbnailSize::Medium).unwrap();
}

#[test]
fn it_creates_large_thumbnails_for_jpeg() {
    create_thumbnail(Jpeg, ThumbnailSize::Large).unwrap();
}

#[test]
fn it_creates_small_thumbnails_for_webp() {
    create_thumbnail(Webp, ThumbnailSize::Small).unwrap();
}

#[test]
fn it_creates_medium_thumbnails_for_webp() {
    create_thumbnail(Webp, ThumbnailSize::Medium).unwrap();
}

#[test]
fn it_creates_large_thumbnails_for_webp() {
    create_thumbnail(Webp, ThumbnailSize::Large).unwrap();
}
#[test]
fn it_creates_small_thumbnails_for_png_unknown() {
    create_thumbnails_unknown(Png, ThumbnailSize::Small).unwrap();
}

#[test]
fn it_creates_medium_thumbnails_for_png_unknown() {
    create_thumbnails_unknown(Png, ThumbnailSize::Medium).unwrap();
}

#[test]
fn it_creates_large_thumbnails_for_png_unknown() {
    create_thumbnails_unknown(Png, ThumbnailSize::Large).unwrap();
}

#[test]
fn it_creates_small_thumbnails_for_jpeg_unknown() {
    create_thumbnails_unknown(Jpeg, ThumbnailSize::Small).unwrap();
}

#[test]
fn it_creates_medium_thumbnails_for_jpeg_unknown() {
    create_thumbnails_unknown(Jpeg, ThumbnailSize::Medium).unwrap();
}

#[test]
fn it_creates_large_thumbnails_for_jpeg_unknown() {
    create_thumbnails_unknown(Jpeg, ThumbnailSize::Large).unwrap();
}

#[test]
fn it_creates_small_thumbnails_for_webp_unknown() {
    create_thumbnails_unknown(Webp, ThumbnailSize::Small).unwrap();
}

#[test]
fn it_creates_medium_thumbnails_for_webp_unknown() {
    create_thumbnails_unknown(Webp, ThumbnailSize::Medium).unwrap();
}

#[test]
fn it_creates_large_thumbnails_for_webp_unknown() {
    create_thumbnails_unknown(Webp, ThumbnailSize::Large).unwrap();
}

fn create_thumbnail(image_type: ImageType, size: ThumbnailSize) -> ThumbResult<Vec<Thumbnail>> {
    match image_type {
        ImageType::Png => {
            let reader = Cursor::new(PNG_BYTES);
            create_thumbnails(reader, FileFormat::PortableNetworkGraphics, [size])
        }
        ImageType::Jpeg => {
            let reader = Cursor::new(JPG_BYTES);
            create_thumbnails(reader, FileFormat::JointPhotographicExpertsGroup, [size])
        }
        ImageType::Webp => {
            let reader = Cursor::new(WEBP_BYTES);
            create_thumbnails(reader, FileFormat::Webp, [size])
        }
    }
}
fn create_thumbnails_unknown(
    image_type: ImageType,
    size: ThumbnailSize,
) -> ThumbResult<Vec<Thumbnail>> {
    match image_type {
        ImageType::Png => {
            let mut reader = Cursor::new(PNG_BYTES);
            create_thumbnails_unknown_type(reader, [size])
        }
        ImageType::Jpeg => {
            let mut reader = Cursor::new(JPG_BYTES);
            create_thumbnails_unknown_type(reader, [size])
        }
        ImageType::Webp => {
            let mut reader = Cursor::new(WEBP_BYTES);
            create_thumbnails_unknown_type(reader, [size])
        }
    }
}
