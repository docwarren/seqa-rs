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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

use crate::tabix::caller::Caller;
use crate::traits::feature::Feature;

use crate::models::coordinates::CoordinateSystem;
use crate::tabix::variant_type::VariantType;

/// A single record from a VCF (Variant Call Format) file.
///
/// Fields map directly to the standard VCF columns.  Multi-allelic sites are
/// represented with multiple entries in [`VcfLine::alt_alleles`].
///
/// Coordinates are **1-based closed** as per the VCF specification.
/// Use [`crate::traits::feature::Feature::to_canonical`] to convert to
/// 0-based half-open for interval arithmetic.
#[derive(Debug, Serialize, Deserialize)]
pub struct VcfLine {
    /// CHROM — chromosome/contig name.
    pub chromosome: String,
    /// POS — 1-based position of the first base of the REF allele.
    pub position: u32,
    /// ID — variant identifier (e.g. rsID), or `"."` if absent.
    pub id: String,
    /// REF — reference allele sequence.
    pub ref_allele: String,
    /// ALT — list of alternate allele sequences.
    pub alt_alleles: Vec<String>,
    /// QUAL — Phred-scaled variant quality score, or `None` if the field was `"."`.
    pub quality: Option<f32>,
    /// FILTER — list of filter labels (e.g. `["PASS"]`), or empty if the field was `"."`.
    pub filter: Vec<String>,
    /// INFO — key/value pairs parsed from the INFO column.  Flag entries have an empty value.
    pub info: Vec<(String, String)>,
    /// Per-sample genotype data, keyed by the FORMAT column fields.
    /// `sample_data[i]` contains the field/value pairs for the *i*-th sample.
    pub sample_data: Vec<Vec<(String, String)>>,
}

impl VcfLine {
    /// Try to get an INFO field string value
    fn get_info_val(&self, key: &str) -> Option<String> {
        let val_set: HashMap<String, String> = self
            .info
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        val_set.get(key).map(|v| v.to_string())
    }
    /// Parses a single tab-delimited VCF data line (not a header line) into a [`VcfLine`].
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when the line has fewer than 8 fields or contains
    /// an unparseable position or quality value.
    pub fn from_line(line: String) -> Result<VcfLine, String> {
        let tokens = line.split('\t').collect::<Vec<&str>>();
        if tokens.len() < 8 {
            return Err(format!("Invalid VCF line: {}", line));
        }
        let chromosome = tokens[0].to_string();
        let position = match tokens[1].parse::<u32>() {
            Ok(pos) => pos,
            Err(_) => {
                return Err(format!("Invalid position in VCF line: {}", line));
            }
        };
        let id = tokens[2].to_string();
        let ref_allele = tokens[3].to_string();
        let alt_alleles = tokens[4]
            .split(',')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        // Parse the quality field
        let quality = if tokens[5] == "." {
            None
        } else {
            Some(match tokens[5].parse::<f32>() {
                Ok(q) => q,
                Err(_) => {
                    return Err(format!("Invalid quality in VCF line: {}", line));
                }
            })
        };

        // Parse the filter field
        let filter = if tokens[6] == "." {
            Vec::new()
        } else {
            tokens[6]
                .split(';')
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        };

        // Parse the INFO field
        let info = tokens[7]
            .split(';')
            .map(|s| {
                let parts: Vec<&str> = s.split('=').collect();
                if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    (parts[0].to_string(), String::new())
                }
            })
            .collect::<Vec<(String, String)>>();

        let mut sample_data = Vec::new();

        // If there are sample data, parse them
        // The format is expected to be in the 9th column onwards
        if tokens.len() > 8 {
            let format_keys = tokens[8].split(':').collect::<Vec<&str>>();

            for sample in tokens[9..].iter() {
                let sample_values = sample.split(':').collect::<Vec<&str>>();
                let mut sample_map = Vec::new();
                for (i, key) in format_keys.iter().enumerate() {
                    if i < sample_values.len() {
                        sample_map.push((key.to_string(), sample_values[i].to_string()));
                    } else {
                        sample_map.push((key.to_string(), String::new()));
                    }
                }
                sample_data.push(sample_map);
            }
        }

        Ok(VcfLine {
            chromosome,
            position,
            id,
            ref_allele,
            alt_alleles,
            quality,
            filter,
            info,
            sample_data,
        })
    }
}

impl Display for VcfLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let quality = match self.quality {
            Some(q) => q.to_string(),
            None => ".".to_string(),
        };

        let info = self
            .info
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.to_string()
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect::<Vec<String>>()
            .join(";");

        let format = if self.sample_data.is_empty() {
            String::new()
        } else {
            self.sample_data[0]
                .iter()
                .map(|(k, _)| k.clone())
                .collect::<Vec<String>>()
                .join(":")
        };

        let mut samples = Vec::new();
        for sample in &self.sample_data {
            let sample_values = sample
                .iter()
                .map(|(_, v)| v.to_string())
                .collect::<Vec<String>>()
                .join(":");
            samples.push(sample_values);
        }

        let sample_str = samples.join("\t");

        write!(
            f,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.chromosome,
            self.position,
            self.id,
            self.ref_allele,
            self.alt_alleles.join(","),
            quality,
            self.filter.join(";"),
            info,
            format,
            sample_str
        )
    }
}

impl Feature for VcfLine {
    fn get_chromosome(&self) -> String {
        self.chromosome.clone()
    }
    // If there are multiple alts all values
    // are calculated based on the longest alt allele
    // which is the most significant variant
    // i.e. the one with the largest length difference
    fn get_begin(&self) -> u32 {
        let prefix_len = self.prefix_len().unwrap_or(0);
        if prefix_len == 0 {
            return self.position;
        } else {
            return self.position + prefix_len - 1;
        }
    }

    fn get_end(&self) -> u32 {
        if let Ok(v_type) = self.get_variant_type() {
            return match v_type {
                VariantType::CnGain
                | VariantType::CnLoss
                | VariantType::CnRef
                | VariantType::IndelDel
                | VariantType::SvDel
                | VariantType::SvDup => self.get_begin() + self.get_length(),
                _ => self.get_begin(),
            };
        } else {
            return self.position;
        }
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_length(&self) -> u32 {
        // try SVLEN in info
        if let Some(length) = self.get_info_val("SVLEN") {
            if let Ok(parsed) = length.parse::<u32>() {
                return parsed;
            }
        }
        // try CNVLEN in info
        else if let Some(length) = self.get_info_val("CNVLEN") {
            if let Ok(parsed) = length.parse::<u32>() {
                return parsed;
            }
        }
        // try END
        else if let Some(end_result) = self.get_info_val("END") {
            if let Ok(end) = end_result.parse::<u32>() {
                return (end - self.position) + 1;
            }
        }
        // use longest alt
        else if let Some(alt) = self.longest_alt() {
            if alt.len() > self.ref_allele.len() {
                return alt.len() as u32 - self.ref_allele.len() as u32;
            } else {
                return self.ref_allele.len() as u32 - alt.len() as u32;
            }
        }

        return 1;
    }

    fn coordinate_system(&self) -> CoordinateSystem {
        CoordinateSystem::OneBasedClosed
    }
}

impl VcfLine {
    /// Attempt to infer the caller from the ID field.
    /// In the case of Canvas and Manta callers - the ID field starts Canvas: and Manta respectively
    /// With bcftools the id field is either . or an rs number
    pub fn infer_caller(id: &str) -> Caller {
        if id.starts_with("Canvas") {
            Caller::Canvas
        } else if id.starts_with("Manta") {
            Caller::Manta
        } else {
            Caller::Unknown
        }
    }

    /// Get the variant type from the INFO:SVTYPE field
    /// In Canvas this will only return CNV
    /// In Manta this will return either DEL, INS or DUP
    /// In bcftools this will fail
    pub fn get_variant_type_from_info(&self) -> Option<VariantType> {
        match self.get_info_val("SVTYPE") {
            Some(svtype) => {
                if svtype == "DEL" {
                    return Some(VariantType::SvDel);
                } else if svtype == "INS" {
                    return Some(VariantType::SvIns);
                } else if svtype == "DUP" {
                    return Some(VariantType::SvDup);
                } else {
                    return None;
                }
            }
            _ => None,
        }
    }

    /// Canvas reports the variant type in the alt field.
    /// e.g. <CN0>,<CN3>
    /// and in the id field
    /// e.g. Canvas:GAIN:11:.....
    pub fn get_variant_type_from_canvas_id(&self) -> Option<VariantType> {
        if !self.id.starts_with("Canvas:") {
            None
        } else {
            match self.id.split(':').collect::<Vec<&str>>()[1] {
                "REF" => Some(VariantType::CnRef),
                "GAIN" => Some(VariantType::CnGain),
                "LOSS" => Some(VariantType::CnLoss),
                _ => None,
            }
        }
    }
    /// Attempt to infer the variant type from the alt fields
    /// should be ok in a bcftools vcf
    pub fn infer_variant_type_from_alt_length(&self) -> Option<VariantType> {
        if self.id.starts_with("Canvas:") || self.id.starts_with("Manta") {
            return None;
        }

        let alt = self.longest_alt()?;

        if alt.len() > self.ref_allele.len() {
            Some(VariantType::IndelIns)
        } else if alt.len() < self.ref_allele.len() {
            Some(VariantType::IndelDel)
        } else {
            Some(VariantType::Substitution)
        }
    }

    /// Returns the type of the longest alt allele
    /// * @return The type of the variant (insertion, deletion, or substitution).
    /// The type is determined based on the length of the longest alt allele compared to the reference allele.
    /// If the longest alt allele is longer than the reference allele, it is an insertion.
    /// If the longest alt allele is shorter than the reference allele, it is a deletion.
    /// If the longest alt allele is the same length as the reference allele, it is a substitution.
    pub fn get_variant_type(&self) -> Result<VariantType, String> {
        let variant_type: Option<VariantType> = match VcfLine::infer_caller(&self.id) {
            Caller::Manta => self.get_variant_type_from_info(),
            Caller::Canvas => self.get_variant_type_from_canvas_id(),
            _ => self.infer_variant_type_from_alt_length(),
        };
        variant_type.ok_or("Unable to get variant type".to_owned())
    }
    /// Returns the longest alt allele based on the length difference from the reference allele.
    /// If there are multiple alleles with the same length difference, it returns the first one.
    /// * @return The longest alt allele.
    pub fn longest_alt(&self) -> Option<String> {
        self.alt_alleles
            .iter()
            .max_by_key(|alt| (alt.len() as i32 - self.ref_allele.len() as i32).abs())
            .cloned()
    }

    /// Returns the length of the prefix that is common between the longest alt allele and the reference allele.
    /// * @return The length of the common prefix.
    pub fn prefix_len(&self) -> Result<u32, String> {
        if self.alt_alleles[0].starts_with("<") {
            return Ok(0);
        }
        let longest_alt = self
            .longest_alt()
            .ok_or_else(|| "No alt allele found".to_string())?;
        let longest = longest_alt.len();
        let shortest = if longest < self.ref_allele.len() {
            longest
        } else {
            self.ref_allele.len()
        };
        let mut i: u32 = 0;
        while i < shortest as u32
            && longest_alt.chars().nth(i as usize) == self.ref_allele.chars().nth(i as usize)
        {
            i += 1;
        }
        Ok(i)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn should_init_from_string() {
        let vcf_line = "20\t2\t.\tTCG\tTG,T,TCAG\t.\tPASS\tDP=100".to_string();
        let vcf_record = VcfLine::from_line(vcf_line).unwrap();
        assert_eq!(vcf_record.chromosome, "20");
        assert_eq!(vcf_record.position, 2);
        assert_eq!(vcf_record.id, ".");
        assert_eq!(vcf_record.ref_allele, "TCG");
        assert_eq!(vcf_record.alt_alleles, vec!["TG", "T", "TCAG"]);
        assert_eq!(vcf_record.quality, None);
        assert_eq!(vcf_record.filter, vec!["PASS"]);
        assert_eq!(
            vcf_record.info,
            Vec::from([("DP".to_string(), "100".to_string())])
        );
        assert_eq!(vcf_record.sample_data.len(), 0);
        assert_eq!(vcf_record.get_id(), ".");
        assert_eq!(vcf_record.get_begin(), 2);
        assert_eq!(vcf_record.get_end(), 4);
        assert_eq!(vcf_record.get_length(), 2);
    }

    #[test]
    pub fn should_handle_manta_symbolic_alt() {
        let line = "1	14110160	MantaDUP:TANDEM:1137:0:1:0:0:0	G	<DUP:TANDEM>	999	PASS	END=14112070;SVTYPE=DUP;SVLEN=1910;SVINSLEN=38;SVINSSEQ=TCAGCGTCAAGTAGGAGCTGTACTAAAAATTTATGTAA	GT:FT:GQ:PL:PR:SR	0/1:PASS:591:999,0,588:24,19:36,31".to_string();
        let record = VcfLine::from_line(line).unwrap();
        assert_eq!(record.chromosome, "1");
        assert_eq!(record.position, 14110160);
        assert_eq!(record.id, "MantaDUP:TANDEM:1137:0:1:0:0:0");
        assert_eq!(record.ref_allele, "G");
        assert_eq!(record.alt_alleles, vec!["<DUP:TANDEM>"]);
        assert_eq!(record.quality, Some(999 as f32));
        assert_eq!(record.filter, vec!["PASS"]);
        assert_eq!(record.get_info_val("END"), Some("14112070".to_string()));
        assert_eq!(record.get_info_val("SVTYPE"), Some("DUP".to_string()));
        assert_eq!(record.get_variant_type(), Ok(VariantType::SvDup));
        assert_eq!(record.sample_data.len(), 1);
        assert_eq!(record.get_id(), "MantaDUP:TANDEM:1137:0:1:0:0:0".to_owned());
        assert_eq!(record.get_begin(), 14110160);
        assert_eq!(record.get_end(), 14112070);
        assert_eq!(record.get_length(), 1910);
    }

    #[test]
    pub fn should_handle_canvas_symbolic_alt() {
        let line = "1	25274839	Canvas:LOSS:1:25274840-25315204	N	<CN0>	36.58	PASS	SVTYPE=CNV;END=25315204;CNVLEN=40365;CIPOS=-671,671;CIEND=-685,685	GT:RC:BC:CN:MCC:MCCQ:QS:FT	0/1:52.33:4:1:1:.:36.58:PASS".to_string();
        let record = VcfLine::from_line(line).unwrap();
        assert_eq!(record.chromosome, "1");
        assert_eq!(record.position, 25274839);
        assert_eq!(record.id, "Canvas:LOSS:1:25274840-25315204");
        assert_eq!(record.ref_allele, "N");
        assert_eq!(record.alt_alleles, vec!["<CN0>"]);
        assert_eq!(record.quality, Some(36.58 as f32));
        assert_eq!(record.filter, vec!["PASS"]);
        assert_eq!(record.get_info_val("END"), Some("25315204".to_string()));
        assert_eq!(record.get_info_val("SVTYPE"), Some("CNV".to_string()));
        assert_eq!(record.get_variant_type(), Ok(VariantType::CnLoss));
        assert_eq!(record.sample_data.len(), 1);
        assert_eq!(
            record.get_id(),
            "Canvas:LOSS:1:25274840-25315204".to_owned()
        );
        assert_eq!(record.get_begin(), 25_274_839);
        assert_eq!(record.get_end(), 25_315_204);
        assert_eq!(record.get_length(), 40_365);
    }
}
