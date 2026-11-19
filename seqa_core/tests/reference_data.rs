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

use seqa_core::reference_data::{cytobands, gene_coordinates};

const GENE_DATASETS: [(&str, &str); 4] = [
    ("hg38", "refseq"),
    ("hg19", "refseq"),
    ("hg38", "gencode"),
    ("hg19", "gencode"),
];

#[test]
fn key_genes_in_every_dataset() {
    for (genome, source) in GENE_DATASETS {
        let genes = gene_coordinates(genome, source);
        for (symbol, chromosome) in [("BRCA1", "chr17"), ("BRCA2", "chr13"), ("TP53", "chr17")] {
            let gene = genes
                .iter()
                .find(|gene| gene.symbol == symbol)
                .unwrap_or_else(|| panic!("{symbol} missing from {genome}/{source}"));
            assert_eq!(gene.chromosome, chromosome, "{symbol} in {genome}/{source}");
        }
    }
}

#[test]
fn grch37_alias_selects_hg19() {
    let alias = gene_coordinates("GRCh37", "refseq");
    let hg19 = gene_coordinates("hg19", "refseq");
    assert!(std::ptr::eq(alias, hg19));
}

#[test]
fn grch38_selects_hg38() {
    let alias = gene_coordinates("grch38", "refseq");
    let hg38 = gene_coordinates("hg38", "refseq");
    assert!(std::ptr::eq(alias, hg38));
}

fn band_fields(genome: &str, chromosome: &str) -> Vec<(String, u32, u32, String, String)> {
    cytobands(genome, chromosome)
        .into_iter()
        .map(|band| (band.chromosome, band.begin, band.end, band.name, band.stain))
        .collect()
}

#[test]
fn unrecognised_genome_falls_back_to_hg38() {
    assert_eq!(band_fields("mm10", "chr1"), band_fields("hg38", "chr1"));
    assert_ne!(band_fields("mm10", "chr1"), band_fields("hg19", "chr1"));
}

#[test]
fn hg19_and_hg38_differ() {
    let hg19 = gene_coordinates("hg19", "refseq");
    let hg38 = gene_coordinates("hg38", "refseq");
    assert!(!std::ptr::eq(hg19, hg38));
}

#[test]
fn gencode_and_refseq_differ() {
    let gencode = gene_coordinates("hg38", "gencode");
    let refseq = gene_coordinates("hg38", "refseq");
    assert!(!std::ptr::eq(gencode, refseq));
}

#[test]
fn unknown_source_selects_refseq_for_the_genome() {
    let unknown = gene_coordinates("hg19", "ensembl");
    let refseq = gene_coordinates("hg19", "refseq");
    assert!(std::ptr::eq(unknown, refseq));
}

#[test]
fn hg38_chr1_first_band() {
    let bands = cytobands("grch38", "chr1");
    let first = bands.first().expect("hg38 chr1 has no bands");
    assert_eq!(first.chromosome, "chr1");
    assert_eq!(first.begin, 0);
    assert_eq!(first.end, 2300000);
    assert_eq!(first.name, "p36.33");
    assert_eq!(first.stain, "gneg");
}

#[test]
fn hg19_chr1_bands_are_all_on_chr1() {
    let bands = cytobands("hg19", "chr1");
    assert!(!bands.is_empty());
    assert!(bands.iter().all(|band| band.chromosome == "chr1"));
}

#[test]
fn chromosome_match_is_exact() {
    assert!(cytobands("grch38", "1").is_empty());
}

#[test]
fn alt_contig_has_no_cytobands() {
    assert!(cytobands("grch38", "chr1_KI270762v1_alt").is_empty());
}

#[test]
fn chrm_has_no_cytobands() {
    assert!(cytobands("grch38", "chrM").is_empty());
}

#[test]
fn every_band_has_a_name() {
    for genome in ["hg38", "hg19"] {
        let chromosomes = (1..=22)
            .map(|number| format!("chr{number}"))
            .chain(["chrX".to_string(), "chrY".to_string()]);
        for chromosome in chromosomes {
            for band in cytobands(genome, &chromosome) {
                assert!(!band.name.is_empty(), "{genome} {chromosome} band at {} has no name", band.begin);
            }
        }
    }
}

#[test]
fn repeated_lookups_reuse_loaded_data() {
    let first = gene_coordinates("grch38", "refseq");
    let second = gene_coordinates("grch38", "refseq");
    assert!(std::ptr::eq(first, second));
}
