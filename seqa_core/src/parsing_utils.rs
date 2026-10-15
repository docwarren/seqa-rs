use bstr::ByteSlice;

use crate::api::parsing_error::ParsingError;

pub fn split_str<'b>(
    bytes: &'b [u8],
    delim: &str,
    expected_len: Option<usize>,
) -> Result<Vec<&'b [u8]>, ParsingError> {
    let tokens: Vec<&[u8]> = bytes.split_str(delim).collect();

    match expected_len {
        Some(len) => {
            if tokens.len() < len {
                return Err(ParsingError::FormatError(format!(
                    "Expected at least {} columns. Found {}",
                    len,
                    tokens.len()
                )));
            } else {
                Ok(tokens)
            }
        }
        _ => Ok(tokens),
    }
}
