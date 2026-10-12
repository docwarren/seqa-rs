use crate::bam::header::header::BamHeader;

pub trait FromBytes: Sized {
    type Error: std::error::Error;
    fn from_bytes(bytes: &[u8], i: usize) -> Result<(Self, usize), Self::Error>;
}