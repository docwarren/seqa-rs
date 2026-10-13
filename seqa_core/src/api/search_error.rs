use thiserror::Error;
use crate::api::parsing_error::ParsingError;
use crate::codecs::codec_error::CodecError;
use crate::stores::error::StoreError;

/// Unified error returned by [`crate::api::search::search_features`] and the
/// format-specific search functions.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Unsupported file format: {0}")]
    UnsupportedFileFormat(String),

    #[error("Invalid options format: {0}")]
    InvalidOptions(String),

    #[error("Invalid Coordinate string format")]
    InvalidCoordinateFormat(String),

    #[error("Error determining index path: {0}")]
    IndexPathError(String),

    #[error("Error determining file type: {0}")]
    PathTypeError(String),

    #[error("Error converting to absolute path: {path}")]
    AbsolutePathError {
        path: String,
        #[source]
        source: std::io::Error
    },

    #[error("Invalid range request ({requested}) => {reason}")]
    InvalidRequest {
        requested: String,
        reason: String
    },

    #[error("Error reading {path}")]
    ReadError {
        path: String,
        #[source]
        source: StoreError
    },

    #[error("Error parsing file {path}")]
    ParseError{
        path: String,
        source: ParsingError
    },

    #[error("Error decompressing {path}")]
    CodecError {
        path: String,
        #[source]
        source: CodecError
    },

    #[error("File not found {path}")]
    NotFound {
        path: String
    },
}