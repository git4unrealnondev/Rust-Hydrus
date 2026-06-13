use file_format::FileFormat;
use image::ImageError;
use std::fmt::{Debug, Display, Formatter};
use std::io;

pub type ThumbResult<T> = Result<T, ThumbError>;

#[derive(Debug)]
pub enum ThumbError {
    IO(io::Error),
    Image(image::error::ImageError),
    Decode,
    Unsupported(FileFormat),
    NullVideo,
    FFMPEG(String),
}

impl Display for ThumbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(_) => write!(f, "an io error occurred"),
            Self::Image(e) => write!(f, "an image error occurred {e}"),
            Self::Decode => write!(f, "failed to decode image"),
            Self::Unsupported(mime) => write!(f, "Unsupported media type {mime}"),
            Self::NullVideo => write!(f, "no video data found in file"),
            Self::FFMPEG(e) => write!(f, "ffmpeg error: {e}"),
        }
    }
}

impl std::error::Error for ThumbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IO(e) => e.source(),
            Self::Image(i) => i.source(),
            _ => None,
        }
    }
}

impl From<io::Error> for ThumbError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<image::error::ImageError> for ThumbError {
    fn from(e: ImageError) -> Self {
        Self::Image(e)
    }
}
