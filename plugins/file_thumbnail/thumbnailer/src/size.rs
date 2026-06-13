/// Represents fixed sizes of a thumbnail
#[derive(Clone, Copy, Debug)]
pub enum ThumbnailSize {
    Icon,
    Small,
    Medium,
    Large,
    Larger,
    Custom((u32, u32)),
}

impl ThumbnailSize {
    pub const fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Icon => (64, 64),
            Self::Small => (128, 128),
            Self::Medium => (256, 256),
            Self::Large => (512, 512),
            Self::Larger => (1024, 1024),
            Self::Custom(size) => *size,
        }
    }
}
