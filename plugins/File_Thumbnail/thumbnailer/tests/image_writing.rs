use file_format::FileFormat;
use std::io::Cursor;
use std::str::FromStr;
use thumbnailer::error::ThumbResult;
use thumbnailer::{create_thumbnails, create_thumbnails_samplefilter, FilterType, ThumbnailSize};

const PNG_BYTES: &[u8] = include_bytes!("assets/test.png");
const JPG_BYTES: &[u8] = include_bytes!("assets/test.jpg");
const WEBP_BYTES: &[u8] = include_bytes!("assets/test.webp");

enum SourceFormat {
    Png,
    Jpeg,
    Webp,
}

enum TargetFormat {
    Png,
    Jpeg,
}

#[test]
fn it_converts_png_thumbnails_for_png() {
    write_thumbnail(SourceFormat::Png, TargetFormat::Png).unwrap();

    for filter in [
        FilterType::Nearest,
        FilterType::Triangle,
        FilterType::CatmullRom,
        FilterType::Gaussian,
        FilterType::Lanczos3,
    ] {
        write_thumbnail_samplefilter(SourceFormat::Png, TargetFormat::Png, filter).unwrap();
    }
}

#[test]
fn it_converts_jpeg_thumbnails_for_png() {
    write_thumbnail(SourceFormat::Png, TargetFormat::Jpeg).unwrap();

    for filter in [
        FilterType::Nearest,
        FilterType::Triangle,
        FilterType::CatmullRom,
        FilterType::Gaussian,
        FilterType::Lanczos3,
    ] {
        write_thumbnail_samplefilter(SourceFormat::Png, TargetFormat::Jpeg, filter).unwrap();
    }
}

#[test]
fn it_converts_png_thumbnails_for_jpeg() {
    write_thumbnail(SourceFormat::Jpeg, TargetFormat::Png).unwrap();

    for filter in [
        FilterType::Nearest,
        FilterType::Triangle,
        FilterType::CatmullRom,
        FilterType::Gaussian,
        FilterType::Lanczos3,
    ] {
        write_thumbnail_samplefilter(SourceFormat::Jpeg, TargetFormat::Png, filter).unwrap();
    }
}

#[test]
fn it_converts_jpeg_thumbnails_for_jpeg() {
    write_thumbnail(SourceFormat::Jpeg, TargetFormat::Jpeg).unwrap();

    for filter in [
        FilterType::Nearest,
        FilterType::Triangle,
        FilterType::CatmullRom,
        FilterType::Gaussian,
        FilterType::Lanczos3,
    ] {
        write_thumbnail_samplefilter(SourceFormat::Jpeg, TargetFormat::Jpeg, filter).unwrap();
    }
}

#[test]
fn it_converts_png_thumbnails_for_webp() {
    write_thumbnail(SourceFormat::Webp, TargetFormat::Png).unwrap();

    for filter in [
        FilterType::Nearest,
        FilterType::Triangle,
        FilterType::CatmullRom,
        FilterType::Gaussian,
        FilterType::Lanczos3,
    ] {
        write_thumbnail_samplefilter(SourceFormat::Webp, TargetFormat::Png, filter).unwrap();
    }
}

#[test]
fn it_converts_jpeg_thumbnails_for_webp() {
    write_thumbnail(SourceFormat::Webp, TargetFormat::Jpeg).unwrap();

    for filter in [
        FilterType::Nearest,
        FilterType::Triangle,
        FilterType::CatmullRom,
        FilterType::Gaussian,
        FilterType::Lanczos3,
    ] {
        write_thumbnail_samplefilter(SourceFormat::Webp, TargetFormat::Jpeg, filter).unwrap();
    }
}

fn write_thumbnail(
    source_format: SourceFormat,
    target_format: TargetFormat,
) -> ThumbResult<Vec<u8>> {
    let thumb = match source_format {
        SourceFormat::Png => {
            let reader = Cursor::new(PNG_BYTES);
            create_thumbnails(
                reader,
                FileFormat::PortableNetworkGraphics,
                [ThumbnailSize::Medium],
            )
            .unwrap()
        }
        SourceFormat::Jpeg => {
            let reader = Cursor::new(JPG_BYTES);
            create_thumbnails(
                reader,
                FileFormat::JointPhotographicExpertsGroup,
                [ThumbnailSize::Medium],
            )
            .unwrap()
        }
        SourceFormat::Webp => {
            let reader = Cursor::new(WEBP_BYTES);
            create_thumbnails(reader, FileFormat::Webp, [ThumbnailSize::Medium]).unwrap()
        }
    }
    .pop()
    .unwrap();

    let mut buf = Cursor::new(Vec::new());
    match target_format {
        TargetFormat::Png => thumb.write_png(&mut buf)?,
        TargetFormat::Jpeg => thumb.write_jpeg(&mut buf, 8)?,
    }

    Ok(buf.into_inner())
}

fn write_thumbnail_samplefilter(
    source_format: SourceFormat,
    target_format: TargetFormat,
    filter: thumbnailer::FilterType,
) -> ThumbResult<Vec<u8>> {
    let thumb = match source_format {
        SourceFormat::Png => {
            let reader = Cursor::new(PNG_BYTES);
            create_thumbnails_samplefilter(
                reader,
                FileFormat::PortableNetworkGraphics,
                [ThumbnailSize::Medium],
                filter,
            )
            .unwrap()
        }
        SourceFormat::Jpeg => {
            let reader = Cursor::new(JPG_BYTES);
            create_thumbnails_samplefilter(
                reader,
                FileFormat::JointPhotographicExpertsGroup,
                [ThumbnailSize::Medium],
                filter,
            )
            .unwrap()
        }
        SourceFormat::Webp => {
            let reader = Cursor::new(WEBP_BYTES);
            create_thumbnails_samplefilter(
                reader,
                FileFormat::Webp,
                [ThumbnailSize::Medium],
                filter,
            )
            .unwrap()
        }
    }
    .pop()
    .unwrap();

    let mut buf = Cursor::new(Vec::new());
    match target_format {
        TargetFormat::Png => thumb.write_png(&mut buf)?,
        TargetFormat::Jpeg => thumb.write_jpeg(&mut buf, 8)?,
    }

    Ok(buf.into_inner())
}
