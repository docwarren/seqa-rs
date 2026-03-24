use std::io::prelude::*;
use std::io;
use flate2::read::GzDecoder;

// Uncompresses a Gz Encoded vector of bytes and returns a u8 vec or error
// Here &[u8] implements Read
pub fn gzip_decompress(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut gz = GzDecoder::new(bytes);
    let result = gz.read_to_end(&mut decompressed);
    match result {
        Ok(_) => Ok(decompressed),
        Err(e) => Err(e),
    }
}