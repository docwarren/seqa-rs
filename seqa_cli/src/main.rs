use clap::{Parser, Subcommand};
use seqa_core::api::search::SearchFeaturesError;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::stores::StoreService;
use seqa_core::utils::ExtensionError;
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
    #[error("Search Error: {0}")]
    SearchError(#[from] SearchFeaturesError),

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

            // Validate reference genome if provided
            if let Some(ref genome) = reference {
                let genome_lower = genome.to_lowercase();
                if genome_lower != "hg38" && genome_lower != "hg19" {
                    error!("Error: Invalid reference genome '{}'. Allowed values: hg38, hg19", genome);
                    std::process::exit(1);
                }
            }

            let mut options = SearchOptions::new(&file, &coordinates);

            // Set genome before coordinates so set_coordinates can use it
            if let Some(ref genome) = reference {
                options = options.set_genome(genome);
            }

            options = match with_header {
                Some(true) => options.set_include_header(true),
                _ => options.set_include_header(false),
            };

            options = match only_header {
                Some(true) => options.set_header_only(true),
                _ => options.set_header_only(false),
            };

            options = options.set_no_cache(no_cache);

            let store_service = StoreService::new();

            match search(&store_service, &options).await {
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

async fn search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<Vec<String>, ApiError> {
    let search_result = store_service.search_features(options).await?;
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
