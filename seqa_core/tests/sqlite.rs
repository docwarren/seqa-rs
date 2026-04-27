#![cfg(feature = "sqlite")]

use seqa_core::sqlite::{connect_genes, genes::{get_gene_coordinates, get_gene_symbols}};

#[test]
fn test_get_gene_coordinates() {
    let conn = connect_genes("grch38").unwrap();
    let gene = get_gene_coordinates(&conn, "BRCA1").unwrap();
    assert_eq!(gene.gene, "BRCA1");
    assert_eq!(gene.chr, "chr17");
    assert_eq!(gene.begin, 43044295);
    assert_eq!(gene.end, 43170327);
}

#[test]
fn test_get_gene() {
    let conn = connect_genes("grch38").unwrap();
    let genes = get_gene_symbols(&conn).unwrap();
    assert_eq!(genes.len(), 29818);
}
