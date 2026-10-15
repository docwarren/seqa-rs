use bstr::ByteSlice;

use crate::api::parsing_error::ParsingError;

pub fn split_str<'b>(bytes: &'b [u8], delim: &str) -> Result<Vec<&'b [u8]>, ParsingError> {
    let tokens: Vec<&[u8]> = bytes.split_str(delim).collect();

    if tokens.len() < 8 {
        return Err(ParsingError::FormatError(format!(
            "Expected at least 8 columns in a vcf file. Found {}",
            tokens.len()
        )));
    }
    Ok(tokens)
}
