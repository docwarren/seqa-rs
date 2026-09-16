use std::array::TryFromSliceError;
use std::num::ParseIntError;
use std::string::FromUtf8Error;
use thiserror::Error;
use crate::indexes::chunk::ChunkError;

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("{reason}: {line}")]
    InvalidHeaderLine {
        line: String,
        reason: String
    },

    #[error("Invalid line in file: {0}")]
    FormatError(String),

    #[error("Insufficient bytes to read header")]
    InsufficientBytes,

    #[error("Error parsing bytes to integer")]
    BytesParseError(#[from] TryFromSliceError),

    #[error("Error parsing UTF8")]
    StringParseError (#[from] FromUtf8Error),

    #[error("Error parsing coordinates: {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error("Error parsing chunk")]
    ChunkError {
        #[source]
        source: ChunkError
    },
}