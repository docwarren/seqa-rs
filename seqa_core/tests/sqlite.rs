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

#![cfg(feature = "sqlite")]

use seqa_core::sqlite::{connect_genes, genes::get_gene_symbols};

#[test]
fn test_get_gene_coordinates() {
    let conn = connect_genes("grch38").unwrap();
    let genes = get_gene_symbols(&conn).unwrap();
    let brca1 = genes.iter().find(|gene| gene.gene == "BRCA1");
    assert!(!brca1.is_none());
    if let Some(brca) = brca1 {
        assert_eq!(brca.gene, "BRCA1");
        assert_eq!(brca.chr, "chr17");
        assert_eq!(brca.begin, 43044295);
        assert_eq!(brca.end, 43170327);
    }
}

#[test]
fn test_get_gene() {
    let conn = connect_genes("grch38").unwrap();
    let genes = get_gene_symbols(&conn).unwrap();
    assert_eq!(genes.len(), 29818);
}
