use crate::error::{ThumbError, ThumbResult};
use crate::formats::image_format::read_image;
use file_format::{FileFormat, Kind};
use image::DynamicImage;
use std::io::{BufRead, Seek};

use crate::formats::video_format::get_video_frame;

pub mod image_format;
pub mod video_format;

/// Reads the buffer content into an image that can be used for thumbnail generation
pub fn get_base_image<R: BufRead + Seek>(reader: R, mime: FileFormat) -> ThumbResult<DynamicImage> {
    match mime.kind() {
        Kind::Image => read_image(reader, mime),
        Kind::Video => get_video_frame(reader, mime),
        Kind::Other => match mime {
            FileFormat::Mpeg4Part14 => get_video_frame(reader, mime),
            _ => Err(ThumbError::Unsupported(mime)),
        },
        _ => Err(ThumbError::Unsupported(mime)),
    }
}
