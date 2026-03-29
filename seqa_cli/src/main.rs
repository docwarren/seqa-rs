use clap::{CommandFactory, Parser, Subcommand};
use seqa_core::api::bam_search::{BamError, bam_search};
use seqa_core::api::bigbed_search::{BigbedError, bigbed_search};
use seqa_core::api::bigwig_search::{BigwigError, bigwig_search};
use seqa_core::api::fasta_search::{FastaSearchError, fasta_search};
use seqa_core::api::output_format::OutputFormat;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::api::search_result::SearchResult;
use seqa_core::api::tabix_search::{TabixSearchError, tabix_search};
use seqa_core::utils::{format_file_path, get_index_path, get_output_format, ExtensionError};
use std::io::{self, Write};
use thiserror::Error;
use log::{debug, error};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Search {
        /// Path to a file to make a genomic range request against
        /// The file should be one of the following formats:
        /// - BAM
        /// - vcf
        /// - gff
        /// - bed
        /// - gtf
        /// - bedgraph
        file: String,

        /// Genomic coordinates to search for in the file
        /// The format should be "chr:start-end" or "chr:position"
        coordinates: String,

        /// Reference genome build (hg38 or hg19)
        #[arg(short = 'r', long)]
        reference: Option<String>,

        // Include the header in the output
        #[arg(short)]
        with_header: Option<bool>,

        // Only include the header in the output
        #[arg(short)]
        only_header: Option<bool>,

        /// Skip reading from and writing to the local index cache
        #[arg(long)]
        no_cache: bool,
    },
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("BAM Error: {0}")]
    BamError(#[from] BamError),

    #[error("Tabix Error: {0}")]
    TabixError(#[from] TabixSearchError),

    #[error("Fasta Error: {0}")]
    FastaError(#[from] FastaSearchError),

    #[error("Bigwig Error: {0}")]
    BigwigError(#[from] BigwigError),

    #[error("Bigbed Error: {0}")]
    BigbedError(#[from] BigbedError),

    #[error("Extension Error: {0}")]
    ExtensionError(#[from] ExtensionError),
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            err.print().expect("Error writing Error");
            std::process::exit(1);
        }
    };
    match cli.command {
        Commands::Search {
            file,
            coordinates,
            reference,
            with_header,
            only_header,
            no_cache,
        } => {

            let file_path = format_file_path(&file).unwrap_or_else(|e| {
                print_error(&format!("Failed to format file path ({:?})", &file), &e);
                std::process::exit(1);
            });

            let index_path = get_index_path(&file_path).unwrap_or_else(|e| {
                print_error(&format!("Failed to get index path ({:?})", &file_path), &e);
                std::process::exit(1);
            });

            // Validate reference genome if provided
            if let Some(ref genome) = reference {
                let genome_lower = genome.to_lowercase();
                if genome_lower != "hg38" && genome_lower != "hg19" {
                    error!("Error: Invalid reference genome '{}'. Allowed values: hg38, hg19", genome);
                    std::process::exit(1);
                }
            }

            let mut options = SearchOptions::new()
                .set_file_path(&file_path)
                .set_index_path(&index_path);

            // Set genome before coordinates so set_coordinates can use it
            if let Some(ref genome) = reference {
                options = options.set_genome(genome);
            }

            options = options.set_coordinates(&coordinates);

            let output_format = get_output_format(&file_path).unwrap_or_else(|e| {
                print_error(&format!("Failed to get output format ({})", &file_path), &e);
                std::process::exit(1);
            });

            options.set_output_format(&output_format.to_string());

            options = match with_header {
                Some(true) => options.set_include_header(true),
                _ => options.set_include_header(false),
            };

            options = match only_header {
                Some(true) => options.set_header_only(true),
                _ => options.set_header_only(false),
            };

            options = options.set_no_cache(no_cache);

            match search(&options).await {
                Ok(lines) => {
                    let result = print_output(&lines);
                    match result {
                        Ok(_) => {}
                        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                            // Handle broken pipe error gracefully
                            std::process::exit(0);
                        }
                        Err(e) => error!("Error writing output: {:?}", e),
                    }
                }
                Err(e) => {
                    debug!("{:?}", e);
                }
            }
        }
    }
}

fn print_error(message: &str, e: &dyn std::error::Error) {
    error!("Error: {}: {}", message, e);
    let mut cmd = Cli::command();
    cmd.print_help().unwrap();
}

async fn search(options: &SearchOptions) -> Result<Vec<String>, ApiError> {
    let search_result = match options.output_format {
        OutputFormat::BAM => bam_search(options).await?,
        OutputFormat::FASTA => fasta_search(options).await?,
        OutputFormat::BIGWIG => bigwig_search(options).await?,
        OutputFormat::BIGBED => bigbed_search(options).await?,
        OutputFormat::BED |
        OutputFormat::BEDGRAPH |
        OutputFormat::GFF |
        OutputFormat::GTF |
        OutputFormat::VCF => tabix_search(options).await?,
        _ => SearchResult::new()
    };
    Ok(search_result.lines)
}

fn print_output(lines: &Vec<String>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for line in lines {
        writeln!(handle, "{}", line)?;
    }
    Ok(())
}
