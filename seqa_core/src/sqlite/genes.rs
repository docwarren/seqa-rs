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

use rusqlite::{Connection, Result, params};
use thiserror::Error;

use crate::models::{cytoband::Cytoband, gene_coordinate::GeneCoordinate};

#[derive(Debug, Error)]
pub enum GeneError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Parsing error: {0}")]
    ParsingError(#[from] std::num::ParseIntError),

    #[error("Gene not found")]
    NotFound,

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

pub fn establish_connection(db_path: String) -> Result<Connection, GeneError> {
    let conn = Connection::open(db_path).map_err(|e| GeneError::DatabaseError(e))?;
    Ok(conn)
}

pub fn get_gene_coordinates(conn: &Connection, symbol: &str) -> Result<GeneCoordinate, GeneError> {
    let mut stmt = conn
        .prepare("SELECT * FROM coordinates WHERE gene = ?1")
        .map_err(|e| GeneError::DatabaseError(e))?;

    let gene = stmt
        .query_row(params![symbol], |row| {
            let begin_str = row.get::<_, String>(2)?;
            let end_str = row.get::<_, String>(3)?;
            let begin = begin_str.parse::<u32>().map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    32,
                    "begin".to_string(),
                    rusqlite::types::Type::Integer,
                )
            })?;
            let end = end_str.parse::<u32>().map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    32,
                    "end".to_string(),
                    rusqlite::types::Type::Integer,
                )
            })?;

            Ok(GeneCoordinate {
                gene: row.get(0)?,
                chr: row.get(1)?,
                begin,
                end,
            })
        })
        .map_err(|e| GeneError::UnknownError(format!("Failed to query gene coordinates: {}", e)))?;
    Ok(gene)
}

pub fn get_gene_symbols(conn: &Connection) -> Result<Vec<String>, GeneError> {

    let mut stmt = conn.prepare("SELECT gene FROM coordinates")?;
    let gene_result = stmt.query_map([],|row| Ok(row.get(0)?))?;
    let mut genes = Vec::new();
    for gene in gene_result {
        genes.push(gene?);
    }
    Ok(genes)
}

pub fn get_cytobands(conn: &Connection, chromosome: &str) -> Result<Vec<Cytoband>, GeneError> {
    let mut statement = conn
        .prepare("SELECT * FROM cytobands where chromosome = ?1")?;

    let cytoband_iter = statement.query_map(params![format!("{}", chromosome)], |row| {
        Ok(Cytoband {
            chromosome: row.get(0)?,
            begin: (row.get::<_, String>(1)?).parse::<u32>().unwrap(),
            end: (row.get::<_, String>(2)?).parse::<u32>().unwrap(),
            name: row.get(3)?,
            stain: row.get(4)?,
        })
    })?;

    let mut cytobands = Vec::new();
    for cytoband in cytoband_iter {
        if let Ok(cyto) = cytoband {
            cytobands.push(cyto);
        }
    }
    Ok(cytobands)
}
