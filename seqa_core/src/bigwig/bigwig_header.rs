use crate::bigwig::bigwig_data::BigwigData;
use crate::bigwig::bigwig_search::get_header_lines;
use crate::traits::to_lines::ToLines;

pub struct BigWigHeader {}

impl ToLines for BigWigHeader {
    fn to_lines(&self) -> Vec<String>{
        get_header_lines(&BigwigData::COLUMNS, true)
    }
}