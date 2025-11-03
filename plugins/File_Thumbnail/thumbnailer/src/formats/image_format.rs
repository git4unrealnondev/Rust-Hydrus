use crate::error::{ThumbError, ThumbResult};
use file_format::FileFormat;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageFormat};
use std::io::{BufRead, Read, Seek};
use webp::Decoder as WebpDecoder;

/// Reads an image with a known mime type
pub fn read_image<R: BufRead + Seek>(reader: R, format: FileFormat) -> ThumbResult<DynamicImage> {
    match format {
        FileFormat::Webp => read_webp_image(reader),
        _ => read_generic_image(reader, mime_to_image_format(format)),
    }
    /* match mime.essence_str() {
        IMAGE_WEBP_MIME => read_webp_image(reader),
        _ => read_generic_image(reader, mime_to_image_format(mime)),
    }*/
}

/// Reads a webp image
fn read_webp_image<R: Read>(mut reader: R) -> ThumbResult<DynamicImage> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    let webp_image = WebpDecoder::new(&buf)
        .decode()
        .ok_or_else(|| ThumbError::Decode)?;

    Ok(webp_image.to_image())
}

/// Reads a generic image
fn read_generic_image<R: BufRead + Seek>(
    reader: R,
    format: Option<ImageFormat>,
) -> ThumbResult<DynamicImage> {
    let reader = if let Some(format) = format {
        ImageReader::with_format(reader, format)
    } else {
        ImageReader::new(reader).with_guessed_format()?
    };
    let image = reader.decode()?;

    Ok(image)
}

#[allow(clippy::needless_pass_by_value)]
fn mime_to_image_format(mime: FileFormat) -> Option<ImageFormat> {
    match mime {
        FileFormat::PortableNetworkGraphics => Some(ImageFormat::Png),
        FileFormat::JointPhotographicExpertsGroup => Some(ImageFormat::Jpeg),
        FileFormat::WindowsBitmap => Some(ImageFormat::Bmp),
        FileFormat::GraphicsInterchangeFormat => Some(ImageFormat::Gif),
        _ => None,
    }
}
