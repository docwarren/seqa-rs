#![cfg(feature = "sqlite")]

#[test]
#[ignore = "requires local database at /media/drew/ExtraSSD/genes/grch38-genes.db"]
fn test_get_gene_coordinates() {
    use seqa_core::sqlite::establish_connection;
    use seqa_core::sqlite::genes::get_gene_coordinates;

    let conn = establish_connection("/media/drew/ExtraSSD/genes/grch38-genes.db").unwrap();
    let gene = get_gene_coordinates(&conn, "BRCA1").unwrap();
    assert_eq!(gene.gene, "BRCA1");
    assert_eq!(gene.chr, "chr17");
    assert_eq!(gene.begin, 43044295);
    assert_eq!(gene.end, 43170327);
}


#[test]
#[ignore = "requires local database at /media/drew/ExtraSSD/genes/grch38-genes.db"]
fn test_get_gene() {
    use seqa_core::sqlite::establish_connection;
    use seqa_core::sqlite::genes::get_gene_symbols;

    let conn = establish_connection("/media/drew/ExtraSSD/genes/grch38-genes.db").unwrap();
    let genes = get_gene_symbols(&conn).unwrap();
    assert_eq!(genes.len(), 29818);
}
