// Copyright 2026 Seqa23
//
// Author: Andrew Warren
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::codecs::bgzip::BgZipError;

pub struct SubBlock {
    pub s1: u8,
    pub s2: u8,
    pub slen: u16,
    pub bsize: u16
}

pub struct BgZipBlock {
	pub id1: u8,
    pub id2: u8,
    pub cm: u8,
    pub flg: u8,
    pub mtime: u32,
    pub xfl: u8,
    pub os: u8,
    pub xlen: u16,
    pub sub_block: SubBlock,
    pub cdata: Vec<u8>,
    pub crc: u32,
    pub i_size: u32
}

impl BgZipBlock {
    pub fn new() -> Self {
        BgZipBlock {
            id1: 0,
            id2: 0,
            cm: 0,
            flg: 0,
            mtime: 0,
            xfl: 0,
            os: 0,
            xlen: 0,
            sub_block: SubBlock {
                s1: 0,
                s2: 0,
                slen: 0,
                bsize: 0
            },
            cdata: Vec::new(),
            crc: 0,
            i_size: 0
        }
    }

    pub fn from_bytes(bytes: &Vec<u8>, mut i: usize) -> Result<BgZipBlock, BgZipError> {
        let mut bgzip = BgZipBlock::new();
        let start_i = i;
        if bytes.len() - i < 18 {
            return Err(BgZipError::ReadBlockError("Not enough data to read the header".into()));
        }

        // Read the header
        bgzip.id1 = bytes[i];
        i += 1;
        bgzip.id2 = bytes[i];
        i += 1;
        bgzip.cm = bytes[i];
        i += 1;
        bgzip.flg = bytes[i];
        i += 1;
        bgzip.mtime = u32::from_le_bytes(bytes[i..i+4].try_into().map_err(|_| BgZipError::ReadBlockError("Failed to read mtime".into()))?);
        i += 4;
        bgzip.xfl = bytes[i];
        i += 1;
        bgzip.os = bytes[i];
        i += 1;
        bgzip.xlen = u16::from_le_bytes(bytes[i..i+2].try_into().map_err(|_| BgZipError::ReadBlockError("Failed to read xlen".into()))?);
        i += 2;

        let xtra_fields_end = i + bgzip.xlen as usize;

        // Read subblock
        let sub_block = SubBlock {
            s1: bytes[i],
            s2: bytes[i + 1],
            slen: u16::from_le_bytes(bytes[i + 2..i + 4].try_into().map_err(|_| BgZipError::ReadBlockError("Failed to read subfield slen".into()))?),
            bsize: u16::from_le_bytes(bytes[i + 4..i + 6].try_into().map_err(|_| BgZipError::ReadBlockError("Failed to read subfield bsize".into()))?)
        };
        i += 6;
        bgzip.sub_block = sub_block;

        if bytes.len() - start_i < bgzip.sub_block.bsize as usize {
            return Err(BgZipError::ReadBlockError("Not enough data to read the compressed data".into()));
        }
        // Skip the remainder of the extra fields
        while i < xtra_fields_end {
            i += 1;
        }
        // Calculate the size of the cdata
        let cdata_len = bgzip.sub_block.bsize as usize - bgzip.xlen as usize - 19;
        // Read compressed data
        bgzip.cdata.extend_from_slice(&bytes[i..i + cdata_len]);
        i += cdata_len;

        // Read CRC and ISIZE
        bgzip.crc = u32::from_le_bytes(bytes[i..i+4].try_into().map_err(|_| BgZipError::ReadBlockError("Failed to read crc".into()))?);
        i += 4;
        bgzip.i_size = u32::from_le_bytes(bytes[i..i+4].try_into().map_err(|_| BgZipError::ReadBlockError("Failed to read i_size".into()))?);

        Ok(bgzip)
    }
}
