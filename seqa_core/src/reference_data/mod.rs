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

//! Gene coordinates and cytobands for hg19 and hg38, bundled into the binary as CSV.
//!
//! Each dataset is parsed the first time it is requested and kept for the life of the process.

use std::sync::OnceLock;

use crate::models::{cytoband::Cytoband, gene_coordinate::GeneCoordinate};

/// One bundled CSV file and its rows, parsed on first use.
struct Dataset<T: 'static> {
    file_name: &'static str,
    contents: &'static str,
    rows: OnceLock<Vec<T>>,
}

static HG38_REFSEQ_GENES: Dataset<GeneCoordinate> = Dataset {
    file_name: "hg38.refseq.genes.csv",
    contents: include_str!("../../data/hg38.refseq.genes.csv"),
    rows: OnceLock::new(),
};
static HG19_REFSEQ_GENES: Dataset<GeneCoordinate> = Dataset {
    file_name: "hg19.refseq.genes.csv",
    contents: include_str!("../../data/hg19.refseq.genes.csv"),
    rows: OnceLock::new(),
};
static HG38_GENCODE_GENES: Dataset<GeneCoordinate> = Dataset {
    file_name: "hg38.gencode.genes.csv",
    contents: include_str!("../../data/hg38.gencode.genes.csv"),
    rows: OnceLock::new(),
};
static HG19_GENCODE_GENES: Dataset<GeneCoordinate> = Dataset {
    file_name: "hg19.gencode.genes.csv",
    contents: include_str!("../../data/hg19.gencode.genes.csv"),
    rows: OnceLock::new(),
};
static HG38_CYTOBANDS: Dataset<Cytoband> = Dataset {
    file_name: "hg38.cytobands.csv",
    contents: include_str!("../../data/hg38.cytobands.csv"),
    rows: OnceLock::new(),
};
static HG19_CYTOBANDS: Dataset<Cytoband> = Dataset {
    file_name: "hg19.cytobands.csv",
    contents: include_str!("../../data/hg19.cytobands.csv"),
    rows: OnceLock::new(),
};

/// Returns every gene in the bundled dataset for `genome` and `source`.
///
/// `genome` is matched case-insensitively: `hg19`, `grch37` and `ch37` select hg19, and
/// anything else selects hg38. `source` is `refseq` or `gencode`; any other value selects
/// RefSeq for the chosen genome. Rows come back in the order they appear in the bundled data.
pub fn gene_coordinates(genome: &str, source: &str) -> &'static [GeneCoordinate] {
    let dataset = match (normalize_genome(genome), source) {
        ("hg19", "gencode") => &HG19_GENCODE_GENES,
        ("hg19", _) => &HG19_REFSEQ_GENES,
        (_, "gencode") => &HG38_GENCODE_GENES,
        _ => &HG38_REFSEQ_GENES,
    };
    dataset
        .rows
        .get_or_init(|| parse_genes(dataset.file_name, dataset.contents))
}

/// Returns every bundled cytoband for `genome` on `chromosome`.
///
/// `genome` is matched as in [`gene_coordinates`]. `chromosome` must match exactly (`chr1`,
/// not `1`). A chromosome with no bands returns an empty list.
pub fn cytobands(genome: &str, chromosome: &str) -> Vec<Cytoband> {
    let dataset = match normalize_genome(genome) {
        "hg19" => &HG19_CYTOBANDS,
        _ => &HG38_CYTOBANDS,
    };
    dataset
        .rows
        .get_or_init(|| parse_cytobands(dataset.file_name, dataset.contents))
        .iter()
        .filter(|band| band.chromosome == chromosome)
        .cloned()
        .collect()
}

fn normalize_genome(genome: &str) -> &'static str {
    match genome.to_ascii_lowercase().as_str() {
        "grch37" | "hg19" | "ch37" => "hg19",
        _ => "hg38",
    }
}

/// Parses `chromosome,begin,end,symbol` rows, panicking on a malformed row.
fn parse_genes(file_name: &str, contents: &str) -> Vec<GeneCoordinate> {
    contents
        .lines()
        .enumerate()
        .map(|(index, line)| match line.split(',').collect::<Vec<_>>().as_slice() {
            [chromosome, c_begin, c_end, symbol] => GeneCoordinate {
                chromosome: chromosome.to_string(),
                begin: parse_position(file_name, index, c_begin),
                end: parse_position(file_name, index, c_end),
                symbol: symbol.to_string(),
            },
            _ => panic!("{file_name} line {}: expected 4 fields: {line}", index + 1),
        })
        .collect()
}

/// Parses `chromosome,begin,end,name,stain` rows, panicking on a malformed row.
fn parse_cytobands(file_name: &str, contents: &str) -> Vec<Cytoband> {
    contents
        .lines()
        .enumerate()
        .map(|(index, line)| match line.split(',').collect::<Vec<_>>().as_slice() {
            [chromosome, c_begin, c_end, name, stain] => Cytoband {
                chromosome: chromosome.to_string(),
                begin: parse_position(file_name, index, c_begin),
                end: parse_position(file_name, index, c_end),
                name: name.to_string(),
                stain: stain.to_string(),
            },
            _ => panic!("{file_name} line {}: expected 5 fields: {line}", index + 1),
        })
        .collect()
}

fn parse_position(file_name: &str, index: usize, value: &str) -> u32 {
    value
        .parse()
        .unwrap_or_else(|_| panic!("{file_name} line {}: invalid position {value:?}", index + 1))
}
