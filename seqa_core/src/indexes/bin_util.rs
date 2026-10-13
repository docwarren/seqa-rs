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

use crate::coordinate::Coordinate;
use super::constants::MAX_BIN_SIZE;

fn update_bins(mut k: u32, max_k: u32, bins: &mut [u32; MAX_BIN_SIZE], i: &mut usize) {
    while k <= max_k && *i < MAX_BIN_SIZE {
        bins[*i] = k;
        k += 1;
        *i += 1;
    }
}

pub fn region_to_bins(begin: u32, end: u32, bins: &mut [u32; MAX_BIN_SIZE]) -> usize {
    let mut i: usize = 0;

    bins[i] = 0;
    i += 1;

    let k:u32 = 1 + (begin >> 26);
    let max_k: u32 = 1 + (end >> 26);
    update_bins(k, max_k, bins, &mut i);

    let k:u32 = 9 + (begin >> 23);
    let max_k: u32 = 9 + (end >> 23);
    update_bins(k, max_k, bins, &mut i);

    let k:u32 = 73 + (begin >> 20);
    let max_k: u32 = 73 + (end >> 20);
    update_bins(k, max_k, bins, &mut i);

    let k:u32 = 585 + (begin >> 17);
    let max_k: u32 = 585 + (end >> 17);
    update_bins(k, max_k, bins, &mut i);

    let k:u32 = 4681 + (begin >> 14);
    let max_k: u32 = 4681 + (end >> 14);
    update_bins(k, max_k, bins, &mut i);

    i
}

pub fn get_bin_numbers(coord: &Coordinate) -> Vec<u32> {
    let mut bin_numbers = [0u32; MAX_BIN_SIZE];
    let length = region_to_bins(coord.get_begin(), coord.get_end(), &mut bin_numbers);
    bin_numbers[..length].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexes::test_data::TEST_BINS1;

    #[test]
    fn queries_region_to_bins() {
        let mut bins = [0u32; MAX_BIN_SIZE];
        let begin = 100_000_000;
        let end = 200_000_000;
        
        let length = region_to_bins(begin, end, &mut bins);
        assert_eq!(length, 6981);
        for i in 0..length {
            assert_eq!(bins[i], TEST_BINS1[i] as u32);
        }
    }
}