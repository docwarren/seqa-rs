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

//! Tokenising, matching and ranking.

use crate::Error;
use crate::index::{IndexedSynonym, IndexedTerm, Ontology};

/// Lowercases `text` and splits it into words at every non-alphanumeric character.
pub(crate) fn tokenise(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Joins words with a single space.
pub(crate) fn normalise_words(words: &[String]) -> String {
    words.join(" ")
}

/// A query prepared for matching.
#[derive(Debug)]
pub(crate) struct Query {
    /// Trimmed and lowercased, for ID matching.
    id: String,
    /// Tokenised words, for name and synonym matching.
    words: Vec<String>,
    /// The words joined by single spaces.
    norm: String,
}

impl Query {
    pub(crate) fn new(query: &str) -> Self {
        let words = tokenise(query);
        Query {
            id: query.trim().to_lowercase(),
            norm: normalise_words(&words),
            words,
        }
    }
}

/// True when every query word is a prefix of at least one of `words`.
fn words_match(query: &[String], words: &[String]) -> bool {
    !query.is_empty()
        && query
            .iter()
            .all(|q| words.iter().any(|w| w.starts_with(q.as_str())))
}

/// The best (lowest) ranking tier any field of `t` reaches for `q`, or `None` if nothing
/// matches.
pub(crate) fn match_tier(t: &IndexedTerm, q: &Query, include_synonyms: bool) -> Option<u8> {
    if !q.id.is_empty() {
        if t.id_lower == q.id {
            return Some(1);
        }
        if t.id_lower.starts_with(&q.id) {
            return Some(2);
        }
        if t.alt_ids_lower.contains(&q.id) {
            return Some(3);
        }
        if t.alt_ids_lower.iter().any(|a| a.starts_with(&q.id)) {
            return Some(4);
        }
    }
    if q.words.is_empty() {
        return None;
    }
    if t.name_norm == q.norm {
        return Some(5);
    }
    if t.name_norm.starts_with(&q.norm) {
        return Some(6);
    }
    let synonyms: &[IndexedSynonym] = if include_synonyms { &t.synonyms } else { &[] };
    if synonyms.iter().any(|s| s.norm == q.norm) {
        return Some(7);
    }
    if synonyms.iter().any(|s| s.norm.starts_with(&q.norm)) {
        return Some(8);
    }
    if words_match(&q.words, &t.name_words)
        || synonyms.iter().any(|s| words_match(&q.words, &s.words))
    {
        return Some(9);
    }
    None
}

/// Minimum number of letters or digits a query needs before it is searched.
pub const MIN_QUERY_CHARS: usize = 3;

/// Returns `true` when `query` has fewer than [`MIN_QUERY_CHARS`] letters or digits
/// (Unicode alphanumeric characters; spaces and punctuation don't count).
///
/// [`search`](crate::search) returns an empty list for such queries without matching
/// IDs, names or synonyms. Hosts can call this to show a hint instead.
///
/// ```
/// assert!(seqa_obos::is_query_too_short("hp:"));
/// assert!(seqa_obos::is_query_too_short("a b"));
/// assert!(!seqa_obos::is_query_too_short("hp:0"));
/// ```
pub fn is_query_too_short(query: &str) -> bool {
    query
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(MIN_QUERY_CHARS)
        .count()
        < MIN_QUERY_CHARS
}

/// Options for [`search`](crate::search).
///
/// `SearchOptions::default()` matches synonyms and returns at most 100 results. Use
/// struct update syntax to change one field:
/// `SearchOptions { include_synonyms: false, ..Default::default() }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    /// Also match EXACT synonyms (ranking tiers 7 and 8, and word matches in tier 9).
    /// On by default. When off, only IDs, alt IDs and names are matched. NARROW, BROAD
    /// and RELATED synonyms are never matched, but every synonym is still
    /// included in the returned [`Term`](crate::Term)s.
    pub include_synonyms: bool,
    /// Maximum number of results, taken after ranking. Default 100; 0 is rejected
    /// with [`Error::InvalidArgument`].
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            include_synonyms: true,
            limit: 100,
        }
    }
}

/// Rejects a zero limit with `InvalidArgument`.
pub(crate) fn check_options(options: &SearchOptions) -> Result<(), Error> {
    if options.limit == 0 {
        return Err(Error::InvalidArgument("limit must be at least 1".into()));
    }
    Ok(())
}

/// Matches, ranks and truncates to `options.limit`. Returns no results for a
/// too-short query. Callers check the options first with [`check_options`].
pub(crate) fn search_indexed<'a>(
    ontology: &'a Ontology,
    query: &str,
    options: &SearchOptions,
) -> Vec<&'a IndexedTerm> {
    if is_query_too_short(query) {
        return Vec::new();
    }
    let q = Query::new(query);
    let mut hits: Vec<(u8, &IndexedTerm)> = ontology
        .terms
        .iter()
        .filter_map(|t| match_tier(t, &q, options.include_synonyms).map(|r| (r, t)))
        .collect();
    hits.sort_by(|(ra, a), (rb, b)| {
        (ra, &a.name_lower, &a.term.id).cmp(&(rb, &b.name_lower, &b.term.id))
    });
    hits.truncate(options.limit);
    hits.into_iter().map(|(_, t)| t).collect()
}

/// Each term's OBO `[Term]` stanza, as written by `fastobo`.
pub(crate) fn obo_text(terms: &[&IndexedTerm]) -> Vec<String> {
    terms.iter().map(|t| t.frame.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Ontology;
    use crate::test_support::{mini, ontology};

    /// Matching terms (no sorting) as (id, tier), synonyms off.
    fn tiers(o: &Ontology, q: &str) -> Vec<(String, u8)> {
        let q = Query::new(q);
        let mut v: Vec<(String, u8)> = o
            .terms
            .iter()
            .filter_map(|t| match_tier(t, &q, false).map(|r| (t.term.id.clone(), r)))
            .collect();
        v.sort();
        v
    }

    fn tier_of(o: &Ontology, q: &str, id: &str) -> Option<u8> {
        tiers(o, q)
            .into_iter()
            .find(|(i, _)| i == id)
            .map(|(_, t)| t)
    }

    fn s(v: &[(&str, u8)]) -> Vec<(String, u8)> {
        v.iter().map(|(i, t)| (i.to_string(), *t)).collect()
    }

    #[test]
    fn exact_id_is_tier_1() {
        assert_eq!(tier_of(&mini(), "HP:0000252", "HP:0000252"), Some(1));
        assert_eq!(tier_of(&mini(), "  hp:0000252 ", "HP:0000252"), Some(1));
    }

    #[test]
    fn id_prefix_ignoring_case_is_tier_2() {
        assert_eq!(
            tiers(&mini(), "hp:000025"),
            s(&[
                ("HP:000025", 1),
                ("HP:0000252", 2),
                ("HP:0000253", 2),
                ("HP:0000254", 2),
                ("HP:0000255", 2),
                ("HP:0000256", 2),
                ("HP:0000259", 2),
            ])
        );
    }

    #[test]
    fn alt_id_exact_and_prefix() {
        assert_eq!(tiers(&mini(), "HP:0001366"), s(&[("HP:0000252", 3)]));
        assert_eq!(tiers(&mini(), "hp:000136"), s(&[("HP:0000252", 4)]));
    }

    #[test]
    fn bare_local_id_does_not_match() {
        assert_eq!(tiers(&mini(), "0000252"), s(&[]));
    }

    #[test]
    fn obsolete_exact_id_does_not_match() {
        assert_eq!(tiers(&mini(), "HP:0000257"), s(&[]));
    }

    #[test]
    fn id_before_alt_id() {
        let o = ontology("[Term]\nid: X:1\nname: One\n\n[Term]\nid: X:2\nname: Two\nalt_id: X:1\n");
        assert_eq!(tiers(&o, "x:1"), s(&[("X:1", 1), ("X:2", 3)]));
    }

    #[test]
    fn single_word_prefix() {
        let t = tiers(&mini(), "microceph");
        assert!(t.contains(&("HP:0000252".into(), 6)), "{t:?}");
        assert!(t.contains(&("HP:0000253".into(), 9)), "{t:?}");
    }

    #[test]
    fn multi_word_query_in_any_order() {
        let t = tiers(&mini(), "progressive micr");
        let ids: Vec<&str> = t.iter().map(|(i, _)| i.as_str()).collect();
        assert_eq!(ids, ["HP:0000253", "HP:0000254", "HP:0000255"]);
        assert_eq!(tier_of(&mini(), "progressive micr", "HP:0000253"), Some(6));
        assert_eq!(tier_of(&mini(), "progressive micr", "HP:0000255"), Some(9));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(tiers(&mini(), "MICROCEPH"), tiers(&mini(), "microceph"));
    }

    #[test]
    fn punctuation_splits_words_and_mid_word_does_not_match() {
        assert_eq!(tiers(&mini(), "cephaly"), s(&[("HP:0000259", 9)]));
        assert_eq!(tier_of(&mini(), "micro cephaly", "HP:0000259"), Some(5));
    }

    #[test]
    fn exact_name_then_prefix_then_other() {
        let o = mini();
        assert_eq!(tier_of(&o, "microcephaly", "HP:0000252"), Some(5));
        assert_eq!(tier_of(&o, "microcephaly", "HP:0000255"), Some(6));
        assert_eq!(tier_of(&o, "microcephaly", "HP:0000253"), Some(9));
    }

    #[test]
    fn tokenise_splits_on_non_alphanumerics_and_lowercases() {
        assert_eq!(tokenise("Micro-cephaly"), ["micro", "cephaly"]);
        assert_eq!(tokenise("  "), Vec::<String>::new());
        assert_eq!(tokenise("MICROCEPH"), ["microceph"]);
        assert_eq!(
            tokenise("Microcephaly, progressive"),
            ["microcephaly", "progressive"]
        );
    }

    #[test]
    fn normalise_joins_words_with_one_space() {
        assert_eq!(
            normalise_words(&tokenise("  Microcephaly,,  progressive! ")),
            "microcephaly progressive"
        );
    }

    fn ids(o: &Ontology, q: &str, opts: &SearchOptions) -> Vec<String> {
        search_indexed(o, q, opts)
            .iter()
            .map(|t| t.term.id.clone())
            .collect()
    }

    fn no_syn(limit: usize) -> SearchOptions {
        SearchOptions {
            include_synonyms: false,
            limit,
        }
    }

    fn many(n: usize) -> Ontology {
        let src: String = (0..n)
            .map(|i| format!("[Term]\nid: X:{i:04}\nname: Abnormal thing {i:04}\n\n"))
            .collect();
        ontology(&src)
    }

    #[test]
    fn default_options() {
        let o = SearchOptions::default();
        assert!(o.include_synonyms);
        assert_eq!(o.limit, 100);
        assert_eq!(MIN_QUERY_CHARS, 3);
    }

    #[test]
    fn default_limit_is_100() {
        let o = many(250);
        let all = ids(
            &o,
            "abnormal",
            &SearchOptions {
                limit: 1000,
                ..Default::default()
            },
        );
        assert_eq!(all.len(), 250);
        assert_eq!(ids(&o, "abnormal", &SearchOptions::default()), all[..100]);
    }

    #[test]
    fn custom_limit() {
        let o = many(250);
        let all = ids(
            &o,
            "abnormal",
            &SearchOptions {
                limit: 1000,
                ..Default::default()
            },
        );
        assert_eq!(
            ids(
                &o,
                "abnormal",
                &SearchOptions {
                    limit: 20,
                    ..Default::default()
                }
            ),
            all[..20]
        );
    }

    #[test]
    fn zero_limit_is_invalid_argument() {
        let err = check_options(&no_syn(0)).unwrap_err();
        assert!(matches!(err, crate::Error::InvalidArgument(_)), "{err:?}");
    }

    #[test]
    fn short_queries() {
        for q in ["hp", "hp:", "a b", "   ", "", "ab!", "--x--y"] {
            assert!(is_query_too_short(q), "{q:?}");
            assert!(
                search_indexed(&mini(), q, &SearchOptions::default()).is_empty(),
                "{q:?}"
            );
        }
        for q in ["abn", "hp:0", "a b c", "  é1x "] {
            assert!(!is_query_too_short(q), "{q:?}");
        }
        assert!(!ids(&mini(), "abn", &SearchOptions::default()).is_empty());
        assert!(!ids(&mini(), "hp:0", &SearchOptions::default()).is_empty());
    }

    #[test]
    fn exact_id_first() {
        assert_eq!(ids(&mini(), "HP:000025", &no_syn(100))[0], "HP:000025");
    }

    #[test]
    fn id_before_alt_id_in_results() {
        let o =
            ontology("[Term]\nid: X:12\nname: Aaa\nalt_id: X:11\n\n[Term]\nid: X:11\nname: Zzz\n");
        assert_eq!(ids(&o, "x:11", &no_syn(100)), ["X:11", "X:12"]);
    }

    #[test]
    fn exact_name_then_prefix_with_ties_by_name_then_other() {
        let r = ids(&mini(), "microcephaly", &no_syn(100));
        assert_eq!(r[..3], ["HP:0000252", "HP:0000256", "HP:0000255"]);
        assert!(r.contains(&"HP:0000253".to_string()));
        assert!(r.iter().position(|i| i == "HP:0000253") > Some(2));
    }

    #[test]
    fn ties_sorted_by_lowercase_name_then_id() {
        let o = ontology(
            "[Term]\nid: X:3\nname: beta thing\n\n[Term]\nid: X:2\nname: Alpha thing\n\n\
             [Term]\nid: X:1\nname: alpha thing\n",
        );
        assert_eq!(ids(&o, "thing", &no_syn(100)), ["X:1", "X:2", "X:3"]);
    }

    #[test]
    fn obo_text_of_results() {
        let o = mini();
        let hits = search_indexed(&o, "HP:0000252", &no_syn(1));
        let text = obo_text(&hits);
        assert_eq!(text.len(), 1);
        assert!(
            text[0].starts_with("[Term]\nid: HP:0000252\nname: Microcephaly\n"),
            "{}",
            text[0]
        );
        assert!(
            text[0].contains("is_a: HP:0040195 ! Decreased head circumference"),
            "{}",
            text[0]
        );
    }

    fn syn_rank(q: &str, id: &str) -> Option<u8> {
        let o = mini();
        let t = o.terms.iter().find(|t| t.term.id == id).unwrap();
        match_tier(t, &Query::new(q), true)
    }

    #[test]
    fn exact_synonym_matched_by_default_and_not_when_off() {
        let o = mini();
        assert!(ids(&o, "small head", &SearchOptions::default()).contains(&"HP:0000252".into()));
        assert!(!ids(&o, "small head", &no_syn(100)).contains(&"HP:0000252".into()));
    }

    #[test]
    fn narrow_synonym_never_matched() {
        assert!(ids(&mini(), "severe micro", &SearchOptions::default()).is_empty());
        assert_eq!(syn_rank("severe", "HP:0000252"), None);
    }

    #[test]
    fn broad_and_related_never_matched() {
        assert!(ids(&mini(), "reduced brain", &SearchOptions::default()).is_empty());
        assert!(ids(&mini(), "braincase anomaly", &SearchOptions::default()).is_empty());
    }

    #[test]
    fn synonym_matches_kept_alongside_name_matches() {
        let r = ids(&mini(), "microcephaly", &SearchOptions::default());
        assert!(r.contains(&"HP:0000252".into()));
        assert!(r.contains(&"HP:0011451".into()), "{r:?}");
    }

    #[test]
    fn name_and_synonym_match_appears_once_as_name_match() {
        let r = ids(&mini(), "microceph", &SearchOptions::default());
        assert_eq!(r.iter().filter(|i| *i == "HP:0000252").count(), 1);
        assert_eq!(syn_rank("microceph", "HP:0000252"), Some(6));
    }

    #[test]
    fn synonym_tiers() {
        assert_eq!(syn_rank("small head", "HP:0000252"), Some(7));
        assert_eq!(syn_rank("small hea", "HP:0000252"), Some(8));
        assert_eq!(syn_rank("head small", "HP:0000252"), Some(9));
    }

    #[test]
    fn name_prefix_before_exact_synonym() {
        let r = ids(&mini(), "small head", &SearchOptions::default());
        assert_eq!(r, ["HP:0040196", "HP:0000252"]);
    }

    #[test]
    fn only_the_exact_synonym_term_matches() {
        // HP:0000261 has EXACT "Tiny skull"; HP:0000262 has it only as NARROW.
        assert_eq!(
            ids(&mini(), "tiny skull", &SearchOptions::default()),
            ["HP:0000261"]
        );
    }

    #[test]
    fn typedefs_are_ignored() {
        assert!(ids(&mini(), "part", &SearchOptions::default()).is_empty());
    }

    #[test]
    fn non_hpo_ontology() {
        let doc = fastobo::from_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mini2.obo"
        ))
        .unwrap();
        let o = Ontology::from_doc(doc);
        assert_eq!(
            ids(&o, "microceph", &SearchOptions::default()),
            ["MONDO:0001149"]
        );
    }
}
