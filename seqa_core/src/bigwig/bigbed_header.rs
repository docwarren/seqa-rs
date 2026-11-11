use crate::bigwig::bigbed_data::BigbedData;
use crate::bigwig::bigwig_search::get_header_lines;
use crate::traits::to_lines::ToLines;

pub struct BigBedHeader {}

impl ToLines for BigBedHeader {
    fn to_lines(&self) -> Vec<String>{
        get_header_lines(&BigbedData::COLUMNS, true)
    }
}