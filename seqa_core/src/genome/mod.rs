/// Chromosome lengths for the hg19 / GRCh37 reference genome.
/// Index order: chr1–chr22, chrX, chrY, chrM (indices 0–24).
const HG19: [u32; 25] = [249250621, 243199373, 198022430, 191154276, 180915260, 171115067, 159138663, 146364022, 141213431, 135534747, 135006516, 133851895, 115169878, 107349540, 102531392, 90354753, 81195210, 78077248, 59128983, 63025520, 48129895, 51304566, 155270560, 59373566, 16571];
/// Chromosome lengths for the hg38 / GRCh38 reference genome.
const HG38: [u32; 25] = [248956422, 242193529, 198295559, 190214555, 181538259, 170805979, 159345973, 145138636, 138394717, 133797422, 135086622, 133275309, 114364328, 107043718, 101991189, 90338345, 83257441, 80373285, 58617616, 64444167, 46709983, 50818468, 156040895, 57227415, 16569];
/// Chromosome lengths for GRCh37 (same as hg19 for chromosomes 1–22, X, Y; chrM differs).
const GRCH37: [u32; 25] = [249250621, 243199373, 198022430, 191154276, 180915260, 171115067, 159138663, 146364022, 141213431, 135534747, 135006516, 133851895, 115169878, 107349540, 102531392, 90354753, 81195210, 78077248, 59128983, 63025520, 48129895, 51304566, 155270560, 59373566, 16569];
/// Chromosome lengths for GRCh38 (same as hg38).
const GRCH38: [u32; 25] = [248956422, 242193529, 198295559, 190214555, 181538259, 170805979, 159345973, 145138636, 138394717, 133797422, 135086622, 133275309, 114364328, 107043718, 101991189, 90338345, 83257441, 80373285, 58617616, 64444167, 46709983, 50818468, 156040895, 57227415, 16569];

/// Converts a 0-based chromosome index (0 = chr1, 22 = chrX, 23 = chrY, 24 = chrM)
/// to its standard `"chrN"` name.  Returns `None` for indices ≥ 25.
pub fn index_to_chr_str(index: usize) -> Option<String> {
    match index {
        0 => Some("chr1".to_string()),
        1 => Some("chr2".to_string()),
        2 => Some("chr3".to_string()),
        3 => Some("chr4".to_string()),
        4 => Some("chr5".to_string()),
        5 => Some("chr6".to_string()),
        6 => Some("chr7".to_string()),
        7 => Some("chr8".to_string()),
        8 => Some("chr9".to_string()),
        9 => Some("chr10".to_string()),
        10 => Some("chr11".to_string()),
        11 => Some("chr12".to_string()),
        12 => Some("chr13".to_string()),
        13 => Some("chr14".to_string()),
        14 => Some("chr15".to_string()),
        15 => Some("chr16".to_string()),
        16 => Some("chr17".to_string()),
        17 => Some("chr18".to_string()),
        18 => Some("chr19".to_string()),
        19 => Some("chr20".to_string()),
        20 => Some("chr21".to_string()),
        21 => Some("chr22".to_string()),
        22 => Some("chrX".to_string()),
        23 => Some("chrY".to_string()),
        24 => Some("chrM".to_string()),
        _ => None,
    }
}

/// Converts a chromosome name to its 0-based index.
///
/// Both `"chr"` prefixed (e.g. `"chr1"`, `"chrX"`) and bare numeric/letter names
/// (e.g. `"1"`, `"X"`, `"M"`) are accepted.  Returns `None` for unrecognised names.
pub fn chr_index(chr: &str) -> Option<usize> {
    match chr {
        "chr1" => Some(0),
        "chr2" => Some(1),
        "chr3" => Some(2),
        "chr4" => Some(3),
        "chr5" => Some(4),
        "chr6" => Some(5),
        "chr7" => Some(6),
        "chr8" => Some(7),
        "chr9" => Some(8),
        "chr10" => Some(9),
        "chr11" => Some(10),
        "chr12" => Some(11),
        "chr13" => Some(12),
        "chr14" => Some(13),
        "chr15" => Some(14),
        "chr16" => Some(15),
        "chr17" => Some(16),
        "chr18" => Some(17),
        "chr19" => Some(18),
        "chr20" => Some(19),
        "chr21" => Some(20),
        "chr22" => Some(21),
        "chrX" => Some(22),
        "chrY" => Some(23),
        "chrM" => Some(24),
        "1" => Some(0),
        "2" => Some(1),
        "3" => Some(2),
        "4" => Some(3),
        "5" => Some(4),
        "6" => Some(5),
        "7" => Some(6),
        "8" => Some(7),
        "9" => Some(8),
        "10" => Some(9),
        "11" => Some(10),
        "12" => Some(11),
        "13" => Some(12),
        "14" => Some(13),
        "15" => Some(14),
        "16" => Some(15),
        "17" => Some(16),
        "18" => Some(17),
        "19" => Some(18),
        "20" => Some(19),
        "21" => Some(20),
        "22" => Some(21),
        "X" => Some(22),
        "Y" => Some(23),
        "M" => Some(24),
        "MT" => Some(24),
        "chrMT" => Some(24),
        _ => None
    }
}

/// Returns the candidate names to try when looking up a contig in a file's
/// index/header.  The user's supplied name is tried first, followed by its
/// `chr`-prefix-flipped form, plus `M`/`MT` aliases for mitochondrial.
///
/// Examples: `"chr1"` → `["chr1", "1"]`; `"1"` → `["1", "chr1"]`;
/// `"chrM"` → `["chrM", "M", "MT", "chrMT"]`; `"MT"` → `["MT", "chrMT", "M", "chrM"]`.
pub fn chromosome_aliases(name: &str) -> Vec<String> {
    let stripped = name.strip_prefix("chr").unwrap_or(name);
    let mut out = Vec::with_capacity(4);
    out.push(name.to_string());
    if name == stripped {
        out.push(format!("chr{}", stripped));
    } else {
        out.push(stripped.to_string());
    }
    match stripped {
        "M" => {
            out.push("MT".to_string());
            out.push("chrMT".to_string());
        }
        "MT" => {
            out.push("M".to_string());
            out.push("chrM".to_string());
        }
        _ => {}
    }
    out
}

/// Returns an array of the maximum chromosome length for each index position across
/// all supported reference builds (hg19, hg38, GRCh37, GRCh38).
///
/// Used as a conservative upper bound when no specific genome build is given.
pub fn get_longest_possible_genome() -> [u32; 25] {
    let mut genome: [u32; 25] = [0; 25];
    for i in 0..25 {
        genome[i] = HG38[i].max(GRCH38[i]).max(HG19[i]).max(GRCH37[i]);
    }
    genome
}

/// Returns the length of `chr` in the given reference build, or `None` if either
/// the chromosome name or the genome name is unrecognised.
///
/// Supported genome names: `"hg19"`, `"hg38"`, `"grch37"`, `"grch38"`,
/// `"ch37"` (alias for GRCh37), `"ch38"` (alias for GRCh38).
pub fn chromosome_len(chr: &str, genome: &str) -> Option<u32> {
    if let Some(index) = chr_index(chr) {
        match genome {
            "hg19" => Some(HG19[index]),
            "hg38" => Some(HG38[index]),
            "grch37" => Some(GRCH37[index]),
            "grch38" => Some(GRCH38[index]),
            "ch37" => Some(GRCH37[index]),
            "ch38" => Some(GRCH38[index]),
            _ => None
        }
    } else {
        None
    }
}

#[test]
fn test_chr_index() {
    assert_eq!(chr_index("chr1"), Some(0));
    assert_eq!(chr_index("chrX"), Some(22));
    assert_eq!(chr_index("chrM"), Some(24));
    assert_eq!(chr_index("1"), Some(0));
    assert_eq!(chr_index("X"), Some(22));
    assert_eq!(chr_index("M"), Some(24));
    assert_eq!(chr_index("chr23"), None);
    assert_eq!(chr_index("chr_unknown"), None);
}

#[test]
fn test_index_to_chr_str() {
    assert_eq!(index_to_chr_str(0), Some("chr1".to_string()));
    assert_eq!(index_to_chr_str(22), Some("chrX".to_string()));
    assert_eq!(index_to_chr_str(24), Some("chrM".to_string()));
    assert_eq!(index_to_chr_str(25), None);
}

#[test]
fn test_get_longest_possible_genome() {
    let longest = get_longest_possible_genome();
    assert_eq!(longest[0], 249250621); // chr1
    assert_eq!(longest[22], 156040895); // chrX
    assert_eq!(longest[24], 16571); // chrM
    assert_eq!(longest.len(), 25);
}

#[test]
fn test_chromosome_aliases() {
    assert_eq!(chromosome_aliases("chr1"), vec!["chr1", "1"]);
    assert_eq!(chromosome_aliases("1"), vec!["1", "chr1"]);
    assert_eq!(chromosome_aliases("chrX"), vec!["chrX", "X"]);
    assert_eq!(chromosome_aliases("X"), vec!["X", "chrX"]);
    assert_eq!(chromosome_aliases("chrM"), vec!["chrM", "M", "MT", "chrMT"]);
    assert_eq!(chromosome_aliases("M"), vec!["M", "chrM", "MT", "chrMT"]);
    assert_eq!(chromosome_aliases("MT"), vec!["MT", "chrMT", "M", "chrM"]);
    assert_eq!(chromosome_aliases("chrMT"), vec!["chrMT", "MT", "M", "chrM"]);
}

#[test]
fn test_chromosome_len() {
    assert_eq!(chromosome_len("chr1", "hg19"), Some(249250621));
    assert_eq!(chromosome_len("chrX", "hg38"), Some(156040895));
    assert_eq!(chromosome_len("chrM", "grch37"), Some(16569));
    assert_eq!(chromosome_len("chr1", "unknown"), None);
    assert_eq!(chromosome_len("chr_unknown", "hg19"), None);
}