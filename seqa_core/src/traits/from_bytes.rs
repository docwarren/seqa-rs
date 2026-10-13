pub trait FromBytes: Sized {
    type Error: std::error::Error;
    fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), Self::Error>;
}
