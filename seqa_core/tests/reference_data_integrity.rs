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

//! Integrity checks on the bundled reference data. Run these after regenerating the CSVs
//! with `scripts/install_all.sh` and before publishing.

use seqa_core::reference_data::{cytobands, gene_coordinates};

const MINIMUM_GENES_PER_DATASET: usize = 40_000;

const STAINS: [&str; 8] = ["gneg", "gpos25", "gpos50", "gpos75", "gpos100", "acen", "gvar", "stalk"];

/// `chr1`–`chr22`, `chrX` and `chrY`.
fn main_chromosomes() -> Vec<String> {
    (1..=22)
        .map(|number| format!("chr{number}"))
        .chain(["chrX".to_string(), "chrY".to_string()])
        .collect()
}

#[test]
fn gene_datasets_pass_integrity_checks() {
    let mut gene_chromosomes = main_chromosomes();
    gene_chromosomes.push("chrM".to_string());

    for (genome, source) in [("hg38", "refseq"), ("hg19", "refseq"), ("hg38", "gencode"), ("hg19", "gencode")] {
        let genes = gene_coordinates(genome, source);
        assert!(
            genes.len() >= MINIMUM_GENES_PER_DATASET,
            "{genome}/{source} has {} genes, expected at least {MINIMUM_GENES_PER_DATASET}",
            genes.len()
        );
        for (index, gene) in genes.iter().enumerate() {
            let row = format!(
                "{genome}/{source} row {}: {},{},{},{}",
                index + 1,
                gene.chromosome,
                gene.begin,
                gene.end,
                gene.symbol
            );
            assert!(gene_chromosomes.contains(&gene.chromosome), "unexpected chromosome in {row}");
            assert!(!gene.symbol.is_empty(), "empty symbol in {row}");
            assert!(gene.begin >= 1, "begin below 1 in {row}");
            assert!(gene.begin <= gene.end, "begin after end in {row}");
        }
    }
}

#[test]
fn cytoband_datasets_pass_integrity_checks() {
    for genome in ["hg38", "hg19"] {
        for chromosome in main_chromosomes() {
            let bands = cytobands(genome, &chromosome);
            assert!(!bands.is_empty(), "{genome} has no bands for {chromosome}");

            let mut previous_end = 0;
            for band in &bands {
                let row = format!(
                    "{genome} {},{},{},{},{}",
                    band.chromosome, band.begin, band.end, band.name, band.stain
                );
                assert!(band.begin < band.end, "begin not before end in {row}");
                assert!(STAINS.contains(&band.stain.as_str()), "unknown stain in {row}");
                assert_eq!(
                    band.begin, previous_end,
                    "gap or overlap before {row}: previous band ended at {previous_end}"
                );
                previous_end = band.end;
            }
        }
    }
}

#[test]
fn cytobands_cover_only_main_chromosomes() {
    for genome in ["hg38", "hg19"] {
        let path = format!("{}/data/{genome}.cytobands.csv", env!("CARGO_MANIFEST_DIR"));
        let rows_in_file = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
            .lines()
            .count();
        let bands_on_main_chromosomes: usize = main_chromosomes()
            .iter()
            .map(|chromosome| cytobands(genome, chromosome).len())
            .sum();
        assert_eq!(
            rows_in_file, bands_on_main_chromosomes,
            "{genome}.cytobands.csv has rows outside chr1-chr22, chrX and chrY"
        );
    }
}
