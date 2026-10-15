use crate::indexes::chunk::ChunkError;
use bstr::Utf8Error;
use std::array::TryFromSliceError;
use std::num::{ParseFloatError, ParseIntError};
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("{reason}: {line}")]
    InvalidHeaderLine { line: String, reason: String },

    #[error("UTF-8 error")]
    Utf8Error(#[from] Utf8Error),

    #[error("Invalid format: {0}")]
    FormatError(String),

    #[error("Insufficient bytes")]
    InsufficientBytes,

    #[error("Error parsing bytes to integer")]
    BytesParseError(#[from] TryFromSliceError),

    #[error("Error parsing UTF8")]
    StringParseError(#[from] FromUtf8Error),

    #[error("Interger parsing error")]
    ParseIntError(#[from] ParseIntError),

    #[error("Float parsing error")]
    ParseFloatError(#[from] ParseFloatError),

    #[error("Error parsing chunk")]
    ChunkError {
        #[source]
        source: ChunkError,
    },
}
