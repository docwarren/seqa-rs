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

use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use seqa_obos::SearchOptions;

const SHORT_QUERY_MSG: &str = "Search queries must contain at least 3 letters or digits.";

const SEARCH_ABOUT: &str = "\
Search an OBO ontology for terms.

Loads the ontology at LOCATION (an s3://, az://, abfs://, abfss://, azure://, gs://, http(s):// \
or file:// URL, or a local path) and prints the non-obsolete terms that match QUERY. Matching ignores case:

  - ID and alt ID: QUERY matches when it is a prefix of the term's id or one of its alt_ids \
(e.g. hp:000025 matches HP:0000250 to HP:0000259).
  - Name: every word of QUERY must be the start of some word in the term name, in any order \
(e.g. 'progressive micr' matches \"Microcephaly, progressive\").
  - Synonyms: EXACT synonyms are matched by default, with the same word rules. \
Use --no-synonyms to match only IDs, alt IDs and names. NARROW, BROAD and RELATED synonyms are never matched.

Queries need at least 3 letters or digits; spaces and punctuation don't count.";

const RANKING: &str = "\
Ranking (each term is listed once, at its best match):
  1. Exact ID
  2. ID prefix
  3. Exact alt ID
  4. Alt ID prefix
  5. Exact name
  6. Name prefix
  7. Exact synonym (EXACT scope only)
  8. Synonym prefix (EXACT scope only)
  9. Any other word match on the name or an EXACT synonym
Ties are sorted by name (ignoring case), then ID.";

#[derive(Parser)]
#[command(name = "obos", version, about = "Search OBO ontologies")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Search an OBO ontology for terms
    #[command(long_about = SEARCH_ABOUT, after_help = RANKING)]
    Search {
        /// Do not match synonyms (EXACT synonyms are matched by default)
        #[arg(long)]
        no_synonyms: bool,
        /// Maximum number of results
        #[arg(long, value_name = "N", default_value = "100")]
        limit: NonZeroUsize,
        /// Print results as a JSON array instead of OBO stanzas
        #[arg(long)]
        json: bool,
        /// Ontology location: URL or local path
        location: String,
        /// Search text (may start with '-')
        #[arg(allow_hyphen_values = true)]
        query: String,
    },
}

fn write_stdout(text: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    out.write_all(text.as_bytes())?;
    out.flush()
}

fn print(text: &str) -> ExitCode {
    match write_stdout(text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn search(
    location: &str,
    query: &str,
    options: &SearchOptions,
    json: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    seqa_obos::load(location).await?;
    if json {
        let results = seqa_obos::search(query, options)?;
        let mut s = serde_json::to_string(&results)?;
        s.push('\n');
        return Ok(s);
    }
    let stanzas = seqa_obos::search_obo_text(query, options)?;
    if stanzas.is_empty() {
        return Ok(String::new());
    }
    let mut s = stanzas
        .iter()
        .map(|t| t.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n\n");
    s.push('\n');
    Ok(s)
}

#[tokio::main]
async fn main() -> ExitCode {
    let Command::Search {
        no_synonyms,
        limit,
        json,
        location,
        query,
    } = Cli::parse().command;
    if seqa_obos::is_query_too_short(&query) {
        if json {
            eprintln!("{SHORT_QUERY_MSG}");
            return print("[]\n");
        }
        return print(&format!("{SHORT_QUERY_MSG}\n"));
    }
    let options = SearchOptions {
        include_synonyms: !no_synonyms,
        limit: limit.get(),
    };
    match search(&location, &query, &options, json).await {
        Ok(text) => print(&text),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
