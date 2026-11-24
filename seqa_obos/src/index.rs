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

//! The in-memory search index built from a parsed OBO document.

use fastobo::ast::{EntityFrame, OboDoc, TermClause, TermFrame};

use crate::search::{normalise_words, tokenise};
use crate::term::{SynonymScope, Term};

/// The searchable form of one ontology: its named, non-obsolete terms.
#[derive(Debug, Default)]
pub(crate) struct Ontology {
    pub(crate) terms: Vec<IndexedTerm>,
}

/// One term with its precomputed search keys.
#[derive(Debug)]
pub(crate) struct IndexedTerm {
    /// Kept for OBO text output.
    pub(crate) frame: TermFrame,
    /// Converted once at load and cloned into results.
    pub(crate) term: Term,
    pub(crate) id_lower: String,
    pub(crate) alt_ids_lower: Vec<String>,
    /// Lowercased name, for sorting within a tier.
    pub(crate) name_lower: String,
    pub(crate) name_norm: String,
    pub(crate) name_words: Vec<String>,
    /// Search keys for EXACT synonyms only.
    pub(crate) synonyms: Vec<IndexedSynonym>,
}

/// Search keys for one EXACT synonym.
#[derive(Debug)]
pub(crate) struct IndexedSynonym {
    pub(crate) norm: String,
    pub(crate) words: Vec<String>,
}

fn is_obsolete(frame: &TermFrame) -> bool {
    frame
        .clauses()
        .iter()
        .any(|l| matches!(l.as_inner(), TermClause::IsObsolete(true)))
}

impl IndexedTerm {
    fn new(frame: TermFrame) -> Option<Self> {
        if is_obsolete(&frame) {
            return None;
        }
        let term = Term::from_frame(&frame)?;
        let name_words = tokenise(&term.name);
        let synonyms = term
            .synonyms
            .iter()
            .filter(|s| s.value.scope == SynonymScope::Exact)
            .map(|s| {
                let words = tokenise(&s.value.text);
                IndexedSynonym {
                    norm: normalise_words(&words),
                    words,
                }
            })
            .collect();
        Some(IndexedTerm {
            id_lower: term.id.to_lowercase(),
            alt_ids_lower: term
                .alt_ids
                .iter()
                .map(|a| a.value.id.to_lowercase())
                .collect(),
            name_lower: term.name.to_lowercase(),
            name_norm: normalise_words(&name_words),
            name_words,
            synonyms,
            term,
            frame,
        })
    }
}

impl Ontology {
    /// Builds the index, consuming (and so dropping) the document.
    pub(crate) fn from_doc(mut doc: OboDoc) -> Self {
        let entities = std::mem::take(AsMut::<Vec<EntityFrame>>::as_mut(&mut doc));
        drop(doc);
        let terms = entities
            .into_iter()
            .filter_map(|e| match e {
                EntityFrame::Term(frame) => IndexedTerm::new(*frame),
                _ => None,
            })
            .collect();
        Ontology { terms }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::mini;

    #[test]
    fn index_keeps_only_named_non_obsolete_terms() {
        let o = mini();
        let ids: Vec<&str> = o.terms.iter().map(|t| t.term.id.as_str()).collect();
        assert!(!ids.contains(&"HP:0000257"), "obsolete term indexed");
        assert!(!ids.contains(&"HP:0000258"), "nameless term indexed");
        assert!(!ids.contains(&"part_of"), "typedef indexed");
        assert!(ids.contains(&"HP:0000252"));
        assert_eq!(o.terms.len(), 15);
    }

    #[test]
    fn index_stores_search_keys() {
        let o = mini();
        let t = o.terms.iter().find(|t| t.term.id == "HP:0000255").unwrap();
        assert_eq!(t.id_lower, "hp:0000255");
        assert_eq!(t.name_lower, "microcephaly, progressive");
        assert_eq!(t.name_norm, "microcephaly progressive");
        assert_eq!(t.name_words, ["microcephaly", "progressive"]);
        assert_eq!(t.frame.id().as_inner().to_string(), "HP:0000255");
        let mc = o.terms.iter().find(|t| t.term.id == "HP:0000252").unwrap();
        assert_eq!(mc.alt_ids_lower, ["hp:0001366"]);
    }

    #[test]
    fn only_exact_synonyms_get_search_keys() {
        use crate::term::SynonymScope;
        let o = mini();
        let mc = o.terms.iter().find(|t| t.term.id == "HP:0000252").unwrap();
        let keys: Vec<(&str, Vec<&str>)> = mc
            .synonyms
            .iter()
            .map(|s| {
                (
                    s.norm.as_str(),
                    s.words.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            keys,
            vec![
                ("small head", vec!["small", "head"]),
                ("microcephalus", vec!["microcephalus"]),
            ]
        );
        let scopes: Vec<SynonymScope> = mc.term.synonyms.iter().map(|s| s.value.scope).collect();
        assert_eq!(
            scopes,
            [
                SynonymScope::Exact,
                SynonymScope::Exact,
                SynonymScope::Narrow,
                SynonymScope::Broad,
                SynonymScope::Related
            ]
        );
    }
}
