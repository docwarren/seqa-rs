use std::fmt::{Display, Formatter};

use crate::indexes::bigwig::{chr_tree::BigwigChrTree, r_tree::overlaps::Overlaps};

#[derive(Debug, Clone)]
pub struct BigwigData {
    pub chromosome_id: u32,
    pub begin: u32,
    pub end: u32,
    pub step: u32,
    pub span: u32,
    pub type_code: u8,
    pub reserved: u8,
    pub item_count: u16,
    pub data: Vec<DataPoint>,
    pub size: usize
}

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub chr_id: u32,
    pub chr_name: String,
    pub begin: u32,
    pub end: u32,
    pub value: f32,
}

pub fn read_type_1_data(bytes: &[u8], chr_id: u32) -> Result<(DataPoint, usize), String> {
    let begin = u32::from_le_bytes(bytes[0..4].try_into().map_err(|e| format!("Invalid begin: {}", e))?);
    let end = u32::from_le_bytes(bytes[4..8].try_into().map_err(|e| format!("Invalid end: {}", e))?);
    let value = f32::from_le_bytes(bytes[8..12].try_into().map_err(|e| format!("Invalid value: {}", e))?);
    Ok((
        DataPoint {
            chr_id,
            chr_name: String::new(),
            begin,
            end,
            value,
        },
        12,
    ))
}

pub fn read_type_2_data(bytes: &[u8], chr_id: u32, span: u32) -> Result<(DataPoint, usize), String> {
    let begin = u32::from_le_bytes(bytes[0..4].try_into().map_err(|e| format!("Invalid begin: {}", e))?);
    let end = begin + span;
    let value = f32::from_le_bytes(bytes[4..8].try_into().map_err(|e| format!("Invalid value: {}", e))?);
    Ok((
        DataPoint {
            chr_id,
            chr_name: String::new(),
            begin,
            end,
            value,
        },
        8,
    ))
}

pub fn read_type_3_data(bytes: &[u8], chr_id: u32, begin: u32, span: u32) -> Result<(DataPoint, usize), String> {
    let end = begin + span;
    let value = f32::from_le_bytes(bytes[0..4].try_into().map_err(|e| format!("Invalid value: {}", e))?);
    Ok((
        DataPoint {
            chr_id,
            chr_name: String::new(),
            begin,
            end,
            value,
        },
        4,
    ))
}

impl BigwigData {

    pub const SIZE: usize = 24; // Size of the BigwigData without its datapoints.

    pub const COLUMNS: [&str; 4] = [
        "chromosome",
        "begin",
        "end",
        "value",
    ];

    pub fn new() -> Self {
        BigwigData {
            chromosome_id: 0,
            begin: 0,
            end: 0,
            step: 0,
            span: 0,
            type_code: 0,
            reserved: 0,
            item_count: 0,
            data: Vec::new(),
            size: 0
        }
    }

    pub fn from_bytes(bytes: &[u8], chr_tree: &BigwigChrTree) -> Result<Self, String> {
        if bytes.len() < 32 {
            return Err("Not enough bytes for a complete BigwigData".into());
        }
        let chromosome_id = u32::from_le_bytes(bytes[0..4].try_into().map_err(|e| format!("Invalid chromosome_id: {}", e))?);
        let name = chr_tree.get_chromosome_name(chromosome_id).ok_or(format!("Chromosome ID {} not found in ChrTree", chromosome_id))?;
        let begin = u32::from_le_bytes(bytes[4..8].try_into().map_err(|e| format!("Invalid begin: {}", e))?);
        let end = u32::from_le_bytes(bytes[8..12].try_into().map_err(|e| format!("Invalid end: {}", e))?);
        let step = u32::from_le_bytes(bytes[12..16].try_into().map_err(|e| format!("Invalid step: {}", e))?);
        let span = u32::from_le_bytes(bytes[16..20].try_into().map_err(|e| format!("Invalid span: {}", e))?);
        let type_code = bytes[20];
        let reserved = bytes[21];
        let item_count = u16::from_le_bytes(bytes[22..24].try_into().map_err(|e| format!("Invalid item_count: {}", e))?);

        let mut offset = BigwigData::SIZE; // Start after the header
        let mut data = Vec::new();

        for i in 0..item_count as u32 {
            let (mut data_point, bytes_read) = if type_code == 1 {
                read_type_1_data(&bytes[offset..], chromosome_id)?
            } else if type_code == 2 {
                read_type_2_data(&bytes[offset..], chromosome_id, span)?
            } else {
                let point_begin = begin + (i * step);
                read_type_3_data(&bytes[offset..], chromosome_id, point_begin, span)?
            };
            offset += bytes_read;
            data_point.chr_name = name.clone();

            data.push(data_point);
        }

        Ok(BigwigData {
            chromosome_id,
            begin,
            end,
            step,
            span,
            type_code,
            reserved,
            item_count,
            data,
            size: offset
        })
    }
}

impl Display for BigwigData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut output = String::new();
        for data_point in &self.data {
            output.push_str(&data_point.to_string());
        }
        write!(f, "{}", output)
    }
}

impl DataPoint {
    pub fn new(chr_id: u32, chr_name: String, begin: u32, end: u32, value: f32) -> Self {
        DataPoint {
            chr_id,
            chr_name,
            begin,
            end,
            value,
        }
    }
}

impl Display for DataPoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{}\t{}\t{}", self.chr_name, self.begin, self.end, self.value)
    }
}

impl Overlaps for DataPoint {
    fn overlaps(&self, chrom_id1: u32, chrom_id2: u32,start: u32, end: u32) -> bool {
        ((chrom_id2 > self.chr_id) || (chrom_id2 == self.chr_id && end >= self.begin)) &&
        ((chrom_id1 < self.chr_id) || (chrom_id1 == self.chr_id && start <= self.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_type_2_data() {
        let begin: u32 = 5000;
        let value: f32 = 3.14;
        let span: u32 = 100;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&begin.to_le_bytes());
        bytes.extend_from_slice(&value.to_le_bytes());

        let (dp, bytes_read) = read_type_2_data(&bytes, 1, span).unwrap();
        assert_eq!(bytes_read, 8);
        assert_eq!(dp.chr_id, 1);
        assert_eq!(dp.begin, 5000);
        assert_eq!(dp.end, 5100);
        assert_eq!(dp.value, 3.14);
    }

    #[test]
    fn test_read_type_3_data() {
        let value: f32 = 2.718;
        let bytes = value.to_le_bytes();

        let (dp, bytes_read) = read_type_3_data(&bytes, 2, 8000, 50).unwrap();
        assert_eq!(bytes_read, 4);
        assert_eq!(dp.chr_id, 2);
        assert_eq!(dp.begin, 8000);
        assert_eq!(dp.end, 8050);
        assert_eq!(dp.value, 2.718);
    }
}
