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

use seqa_core::models::bam::read::Read;

#[cfg(test)]
mod init_mock_ref_tests {
    use super::*;

    #[test]
    fn test_simple_matches() {
        let mock_ref = Read::init_mock_ref("10");
        assert_eq!(mock_ref, "----------");
    }

    #[test]
    fn test_with_single_mismatch() {
        let mock_ref = Read::init_mock_ref("5A4");
        assert_eq!(mock_ref, "-----A----");
    }

    #[test]
    fn test_with_deletion() {
        let mock_ref = Read::init_mock_ref("5^G5");
        assert_eq!(mock_ref, "-----G-----");
    }

    #[test]
    fn test_complex_md_tag() {
        let mock_ref = Read::init_mock_ref("7^C5G4A0G1T2A3G3");
        assert_eq!(mock_ref, "-------C-----G----AG-T--A---G---");
    }

    #[test]
    fn test_consecutive_mismatches_with_zero_counts() {
        let mock_ref = Read::init_mock_ref("46G0G0T0T0A49");
        assert_eq!(mock_ref, "----------------------------------------------GGTTA-------------------------------------------------");
    }
}

#[cfg(test)]
mod init_merged_cigar_string_tests {
    use super::*;

    #[test]
    fn test_simple_match() {
        // 10M with no mismatches: clippedRef = "----------", enhancedRead = "ATCGATCGAT"
        let clipped_ref = "----------";
        let enhanced_read = "ATCGATCGAT";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "10M");
    }

    #[test]
    fn test_soft_clips() {
        // 3S5M: clippedRef = "...-----", enhancedRead = "AAATTTTT"
        let clipped_ref = "...-----";
        let enhanced_read = "AAATTTTT";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "3S5M");
    }

    #[test]
    fn test_trailing_soft_clips() {
        // 5M3S: clippedRef = "-----...", enhancedRead = "TTTTTAAA"
        let clipped_ref = "-----...";
        let enhanced_read = "TTTTTAAA";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "5M3S");
    }

    #[test]
    fn test_insertion() {
        // 5M3I5M: clippedRef = "-----III-----", enhancedRead = "ATCGAXXXTTTTG"
        let clipped_ref = "-----III-----";
        let enhanced_read = "ATCGAXXXTTTTG";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "5M3I5M");
    }

    #[test]
    fn test_deletion() {
        // 5M1D5M with MD 5^G5: clippedRef = "-----G-----", enhancedRead = "ATCGA-TCGAT"
        let clipped_ref = "-----G-----";
        let enhanced_read = "ATCGA-TCGAT";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "5M1DG5M");
    }

    #[test]
    fn test_mismatch() {
        // Single mismatch: ref has A at position 5, read has T
        // clippedRef = "-----A----", enhancedRead = "ATCGATCGAT" (T at position 5)
        let clipped_ref = "-----A----";
        let enhanced_read = "ATCGATCGAT";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "5M1XT4M");
    }

    #[test]
    fn test_consecutive_mismatches() {
        // 1S100M with MD 46G0G0T0T0A49: 5 consecutive mismatches (NNNNN in read)
        let clipped_ref = ".----------------------------------------------GGTTA-------------------------------------------------";
        let enhanced_read = "GGGTCCTAATCCCTCTCTAACTTTCTGAGTTGACAGTATTATGATGTNNNNNACAGCATAGACTTCGAATTCAAATGGAGTGATCAATGTTATGGAAGGGA";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "1S46M5XNNNNN49M");
    }

    #[test]
    fn test_complex_cigar() {
        // 66S4M1I3M1D24M3S with MD 7^C5G4A0G1T2A3G3
        let clipped_ref = "..................................................................----I---C-----G----AG-T--A---G---...";
        let enhanced_read = "ACCAGCGATAAGCGTCCGCACCTACTTTTTTGTGTTTGCAGCAAAAAATTGAAACGACTTAACCTACTTGTTGA-CTATAAAGGCTTGATTCTCTCGTTTTG";
        let merged = Read::init_merged_cigar_string(clipped_ref, enhanced_read);
        assert_eq!(merged, "66S4M1I3M1DC5M1XA4M2XTT1M1XA2M1XC3M1XC3M3S");
    }

    #[test]
    fn test_clipped_ref_and_enhanced_read_same_length() {
        let clipped_ref = "..................................................................----I---C-----G----AG-T--A---G---...";
        let enhanced_read = "ACCAGCGATAAGCGTCCGCACCTACTTTTTTGTGTTTGCAGCAAAAAATTGAAACGACTTAACCTACTTGTTGA-CTATAAAGGCTTGATTCTCTCGTTTTG";
        assert_eq!(clipped_ref.len(), enhanced_read.len());
    }
}
