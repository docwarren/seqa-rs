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
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use log::error;
use seqa_core::api::search::search_features;
use seqa_core::api::search_error::SearchError;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::sqlite::genes::GeneError;
use seqa_core::sqlite::{
    self,
    genes::{self},
};
use seqa_core::stores::StoreService;
use std::io::{self, Write};

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

    /// Look up gene metadata from the local SQLite databases.
    Genes {
        #[command(subcommand)]
        command: GeneCommands,
    },
}

#[derive(Debug, Subcommand)]
enum GeneCommands {
    /// Print the coordinates (chromosome, begin, end) for a gene symbol.
    Coordinates {
        /// Gene symbol (e.g. BRCA2)
        gene: String,

        /// Reference genome build (hg38, hg19, grch37, grch38). Defaults to grch38.
        #[arg(short = 'r', long, default_value = "grch38")]
        reference: String,
    },

    /// Print every gene symbol in the database, one per line.
    Symbols {
        /// Reference genome build (hg38, hg19, grch37, grch38). Defaults to grch38.
        #[arg(short = 'r', long, default_value = "grch38")]
        reference: String,
    },

    /// Print cytobands for a chromosome.
    Cytobands {
        /// Chromosome name (e.g. chr1)
        chromosome: String,

        /// Reference genome build (hg38, hg19, grch37, grch38). Defaults to grch38.
        #[arg(short = 'r', long, default_value = "grch38")]
        reference: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

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
                    error!(
                        "Error: Invalid reference genome '{}'. Allowed values: hg38, hg19",
                        genome
                    );
                    std::process::exit(1);
                }
            }

            let mut options =
                SearchOptions::new(&file, vec![&coordinates]).expect("Invalid coordinates");

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
                        Ok(_) => Ok(()),
                        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                            bail!("Broken pipe! {:?}", e);
                        }
                        Err(e) => bail!("Error writing output: {:?}", e),
                    }
                }
                Err(e) => {
                    bail!("Search Error: {:?}", e);
                }
            }
        }
        Commands::Genes { command } => {
            run_gene_command(command).with_context(|| "Error fetching genes / cytobands")
        }
    }
}

fn run_gene_command(command: GeneCommands) -> Result<(), GeneError> {
    match command {
        GeneCommands::Coordinates { gene, reference } => {
            let conn = sqlite::connect_genes(&reference)?;
            let coord = genes::get_gene_coordinates(&conn, &gene)?;
            println!(
                "{}\t{}\t{}\t{}",
                coord.gene, coord.chr, coord.begin, coord.end
            );
        }
        GeneCommands::Symbols { reference } => {
            let conn = sqlite::connect_genes(&reference)?;
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            for symbol in genes::get_gene_symbols(&conn)? {
                if writeln!(handle, "{}", symbol).is_err() {
                    break;
                }
            }
        }
        GeneCommands::Cytobands {
            chromosome,
            reference,
        } => {
            let conn = sqlite::connect_cytobands(&reference)?;
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            for c in genes::get_cytobands(&conn, &chromosome)? {
                if writeln!(
                    handle,
                    "{}\t{}\t{}\t{}\t{}",
                    c.chromosome, c.begin, c.end, c.name, c.stain
                )
                .is_err()
                {
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<Vec<String>, SearchError> {
    let search_result = search_features(store_service, options).await?;
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
