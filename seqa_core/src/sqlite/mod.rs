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

pub fn establish_connection(db_name: &str) -> Result<Connection> {
    let conn = Connection::open(db_name)?;
    Ok(conn)
}

const GRCH38_GENES: &[u8] = include_bytes!("../../data/grch38-genes.db");
const GRCH37_GENES: &[u8] = include_bytes!("../../data/grch37-genes.db");
const GRCH38_CYTOBANDS: &[u8] = include_bytes!("../../data/grch38-cytobands.db");
const GRCH37_CYTOBANDS: &[u8] = include_bytes!("../../data/grch37-cytobands.db");

pub fn normalize_genome(genome: &str) -> &'static str {
    match genome.to_ascii_lowercase().as_str() {
        "grch37" | "hg19" | "ch37" => "grch37",
        _ => "grch38",
    }
}

fn materialize(name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "seqa-{}-{}",
        env!("CARGO_PKG_VERSION"),
        name
    ));
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

pub fn connect_genes(genome: &str) -> Result<Connection, GeneError> {
    let (name, bytes) = match normalize_genome(genome) {
        "grch37" => ("grch37-genes.db", GRCH37_GENES),
        _ => ("grch38-genes.db", GRCH38_GENES),
    };
    open_embedded(name, bytes)
}

pub fn connect_cytobands(genome: &str) -> Result<Connection, GeneError> {
    let (name, bytes) = match normalize_genome(genome) {
        "grch37" => ("grch37-cytobands.db", GRCH37_CYTOBANDS),
        _ => ("grch38-cytobands.db", GRCH38_CYTOBANDS),
    };
    open_embedded(name, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_genes_grch38_has_brca2() {
        let conn = connect_genes("grch38").unwrap();
        let coord = genes::get_gene_coordinates(&conn, "BRCA2").unwrap();
        assert_eq!(coord.gene, "BRCA2");
        assert!(!coord.chr.is_empty());
        assert!(coord.end > coord.begin);
    }

    #[test]
    fn embedded_genes_grch37_has_brca2() {
        let conn = connect_genes("hg19").unwrap();
        let coord = genes::get_gene_coordinates(&conn, "BRCA2").unwrap();
        assert_eq!(coord.gene, "BRCA2");
    }

    #[test]
    fn embedded_cytobands_grch38_chr1() {
        let conn = connect_cytobands("grch38").unwrap();
        let bands = genes::get_cytobands(&conn, "chr1").unwrap();
        assert!(!bands.is_empty());
    }
}
