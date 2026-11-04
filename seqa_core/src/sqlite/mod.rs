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

pub mod genes;

use rusqlite::{Connection, OpenFlags, Result};
use std::io;
use std::path::PathBuf;

use crate::sqlite::genes::GeneError;

const HG38_REFSEQ_GENES: &[u8] = include_bytes!("../../data/hg38.refseq.gene.db");
const HG19_REFSEQ_GENES: &[u8] = include_bytes!("../../data/hg19.refseq.gene.db");
const HG38_GENCODE_GENES: &[u8] = include_bytes!("../../data/hg38.gencode.gene.db");
const HG19_GENCODE_GENES: &[u8] = include_bytes!("../../data/hg19.gencode.gene.db");
const HG38_CYTOBANDS: &[u8] = include_bytes!("../../data/hg38.cytobands.db");
const HG19_CYTOBANDS: &[u8] = include_bytes!("../../data/hg19.cytobands.db");

pub fn normalize_genome(genome: &str) -> &'static str {
    match genome.to_ascii_lowercase().as_str() {
        "grch37" | "hg19" | "ch37" => "hg19",
        _ => "hg38",
    }
}

fn materialize(name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("seqa-{}-{}", env!("CARGO_PKG_VERSION"), name));
    let needs_write = match path.metadata() {
        Ok(m) => m.len() as usize != bytes.len(),
        Err(_) => true,
    };
    if needs_write {
        std::fs::write(&path, bytes)?;
    }
    Ok(path)
}

fn open_embedded(name: &str, bytes: &[u8]) -> Result<Connection, GeneError> {
    let path = materialize(name, bytes).map_err(|e| {
        GeneError::UnknownError(format!("failed to materialize embedded {}: {}", name, e))
    })?;
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    Connection::open_with_flags(&path, flags).map_err(GeneError::DatabaseError)
}

pub fn connect_genes(genome: &str, source: &str) -> Result<Connection, GeneError> {
    let name: &str = normalize_genome(genome);
    let key = format!("genes_{}_{}", name, source);
    let bytes = match (name, source) {
        ("hg38", "gencode") => HG38_GENCODE_GENES,
        ("hg19", "gencode") => HG19_GENCODE_GENES,
        ("hg38", "refseq") => HG38_REFSEQ_GENES,
        ("hg19", "refseq") => HG19_REFSEQ_GENES,
        _ => HG38_REFSEQ_GENES,
    };
    open_embedded(&key, bytes)
}

pub fn connect_cytobands(genome: &str) -> Result<Connection, GeneError> {
    let name: &str = normalize_genome(genome);
    let key = format!("cytobands_{}", name);
    let bytes = match name {
        "hg38" => HG38_CYTOBANDS,
        _ => HG19_CYTOBANDS
    };
    open_embedded(&key, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_genes_grch38_has_brca2() {
        let conn = connect_genes("grch38", "refseq").unwrap();
        let genes = genes::get_gene_symbols(&conn).unwrap();
        let brca2 = genes.iter().find(|gene| gene.symbol == "BRCA2").unwrap();
        assert_eq!(brca2.symbol, "BRCA2");
        assert!(!brca2.chromosome.is_empty());
        assert!(brca2.end > brca2.begin);
    }

    #[test]
    fn embedded_genes_grch37_has_brca2() {
        let conn = connect_genes("hg19", "refseq").unwrap();
        let genes = genes::get_gene_symbols(&conn).unwrap();
        let brca2 = genes.iter().find(|gene| gene.symbol == "BRCA2").unwrap();
        assert_eq!(brca2.symbol, "BRCA2");
        assert!(!brca2.chromosome.is_empty());
        assert!(brca2.end > brca2.begin);
    }

    #[test]
    fn embedded_cytobands_grch38_chr1() {
        let conn = connect_cytobands("grch38").unwrap();
        let bands = genes::get_cytobands(&conn, "chr1").unwrap();
        assert!(!bands.is_empty());
    }

    #[test]
    fn gencode_hg38_has_brca2() {
        let conn = connect_genes("grch38", "gencode").unwrap();
        let genes = genes::get_gene_symbols(&conn).unwrap();
        let brca2 = genes.iter().find(|gene| gene.symbol == "BRCA2").unwrap();
        assert_eq!(brca2.symbol, "BRCA2");
        assert!(!brca2.chromosome.is_empty());
        assert!(brca2.end > brca2.begin);
    }

    #[test]
    fn gencode_hg19_has_brca2() {
        let conn = connect_genes("hg19", "gencode").unwrap();
        let genes = genes::get_gene_symbols(&conn).unwrap();
        let brca2 = genes.iter().find(|gene| gene.symbol == "BRCA2").unwrap();
        assert_eq!(brca2.symbol, "BRCA2");
        assert!(!brca2.chromosome.is_empty());
        assert!(brca2.end > brca2.begin);
    }

    #[test]
    fn embedded_cytobands_hg19_chr1() {
        let conn = connect_cytobands("hg19").unwrap();
        let bands = genes::get_cytobands(&conn, "chr1").unwrap();
        assert!(!bands.is_empty());
    }
}
