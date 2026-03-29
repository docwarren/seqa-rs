use serde::{ Serialize, Deserialize };
use std::fmt::Display;
use crate::models::bam::read_utils::{get_tag_value, map_tag_type_to_result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Tags {
    pub bytes: Vec<u8>,
}

impl Tags {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Tags { bytes }
    }

    /// Get the value of a specific tag by its 2-character name (e.g., "MD")
    pub fn get_value(&self, tag_name: &str) -> Option<String> {
        if self.bytes.is_empty() || tag_name.len() != 2 {
            return None;
        }

        let tag_bytes: Vec<u8> = tag_name.bytes().collect();
        let mut i = 0;

        while i + 2 < self.bytes.len() {
            // Check if this is the tag we're looking for
            if self.bytes[i] == tag_bytes[0] && self.bytes[i + 1] == tag_bytes[1] {
                i += 2;
                let value_type = self.bytes[i];
                i += 1;

                // Handle array type
                if value_type == b'B' {
                    let val_type = self.bytes[i];
                    i += 1;
                    let count = i32::from_le_bytes(
                        self.bytes[i..i + 4].try_into().ok()?
                    );
                    i += 4;

                    let mut values = Vec::new();
                    for _ in 0..count {
                        let (value, new_i) = map_tag_type_to_result(&self.bytes, i, val_type).ok()?;
                        values.push(value);
                        i = new_i;
                    }
                    return Some(values.join(","));
                } else {
                    let (value, _) = map_tag_type_to_result(&self.bytes, i, value_type).ok()?;
                    return Some(value);
                }
            }

            // Skip to next tag
            let (_, new_i) = get_tag_value(&self.bytes, i).ok()?;
            i = new_i;
        }

        None
    }

    pub fn to_string(&self) -> Result<String, String> {
        if self.bytes.is_empty() {
            return Ok(String::from("*"));
        }

        let mut i = 0;
        let mut result = String::new();

        while i < self.bytes.len() {
            let (tag_str, new_i) = get_tag_value(&self.bytes, i)?;
            i = new_i;

            result.push_str(&tag_str);
            if i < self.bytes.len() {
                result.push('\t'); // Add tab between tags
            }
        }
        Ok(result)
    }
}

impl Display for Tags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_z_tag(name: &str, value: &str) -> Vec<u8> {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(b'Z');
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        bytes
    }

    fn build_i32_tag(name: &str, value: i32) -> Vec<u8> {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(b'i');
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    fn build_a_tag(name: &str, value: u8) -> Vec<u8> {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(b'A');
        bytes.push(value);
        bytes
    }

    #[test]
    fn test_empty_tags() {
        let tags = Tags::from_bytes(vec![]);
        assert_eq!(tags.to_string().unwrap(), "*");
    }

    #[test]
    fn test_single_z_tag() {
        let bytes = build_z_tag("RG", "NA12877");
        let tags = Tags::from_bytes(bytes);
        assert_eq!(tags.to_string().unwrap(), "RG:Z:NA12877");
    }

    #[test]
    fn test_multiple_tags() {
        let mut bytes = build_z_tag("RG", "NA12877");
        bytes.extend(build_a_tag("XT", b'U'));
        bytes.extend(build_i32_tag("NM", 0));
        let tags = Tags::from_bytes(bytes);
        assert_eq!(tags.to_string().unwrap(), "RG:Z:NA12877\tXT:A:U\tNM:i:0");
    }

    #[test]
    fn test_get_value_found() {
        let mut bytes = build_z_tag("RG", "sample1");
        bytes.extend(build_i32_tag("NM", 5));
        let tags = Tags::from_bytes(bytes);
        assert_eq!(tags.get_value("NM"), Some("5".into()));
        assert_eq!(tags.get_value("RG"), Some("sample1".into()));
    }

    #[test]
    fn test_get_value_not_found() {
        let bytes = build_z_tag("RG", "sample1");
        let tags = Tags::from_bytes(bytes);
        assert_eq!(tags.get_value("MD"), None);
    }

    #[test]
    fn test_get_value_empty_tags() {
        let tags = Tags::from_bytes(vec![]);
        assert_eq!(tags.get_value("RG"), None);
    }

    #[test]
    fn test_get_value_invalid_tag_name() {
        let bytes = build_z_tag("RG", "sample1");
        let tags = Tags::from_bytes(bytes);
        assert_eq!(tags.get_value("R"), None);
        assert_eq!(tags.get_value("RGX"), None);
    }

    #[test]
    fn test_display() {
        let bytes = build_z_tag("RG", "NA12877");
        let tags = Tags::from_bytes(bytes);
        assert_eq!(format!("{}", tags), "RG:Z:NA12877");
    }

    #[test]
    fn test_get_value_b_array() {
        let mut bytes = vec![b'B', b'C', b'B', b'C'];
        bytes.extend_from_slice(&3i32.to_le_bytes());
        bytes.extend_from_slice(&[10, 20, 30]);
        let tags = Tags::from_bytes(bytes);
        assert_eq!(tags.get_value("BC"), Some("10,20,30".into()));
    }
}
