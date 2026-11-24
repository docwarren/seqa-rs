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

//! Serializable term entries and conversion from `fastobo` term frames.

use fastobo::ast::{self, Line, TermClause, TermFrame};
use serde::Serialize;

/// A full `[Term]` entry, as returned by [`search`](crate::search).
///
/// Serializes to a JSON object with `id` and `name` strings; every other field is
/// present only when the term has that clause. Repeated clauses keep file order.
/// `is_anonymous` and `builtin` are booleans. Every other field holds clause lines
/// ([`Annotated`]), which carry a `qualifiers` array and a `comment` string when the
/// line has them. `is_obsolete` is not represented because obsolete terms are never
/// returned. `id` and `name` are plain strings, so any qualifiers or trailing comment on
/// those two lines are dropped. For example:
///
/// ```json
/// {"id":"HP:0000252","name":"Microcephaly",
///  "synonyms":[{"text":"Small head","scope":"EXACT","type":"layperson","xrefs":[]}],
///  "is_a":[{"id":"HP:0040195","comment":"Decreased head circumference"}]}
/// ```
///
/// `Term` is `#[non_exhaustive]`: fields may be added in later versions, so it cannot be
/// constructed or exhaustively destructured outside this crate.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Term {
    /// The term's `id`.
    pub id: String,
    /// The term's first `name`.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `is_anonymous`.
    pub is_anonymous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `builtin`.
    pub builtin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `namespace`.
    pub namespace: Option<Annotated<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `comment`.
    pub comment: Option<Annotated<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `created_by`.
    pub created_by: Option<Annotated<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `creation_date`.
    pub creation_date: Option<Annotated<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `def`.
    pub def: Option<Annotated<Definition>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `alt_id` clauses.
    pub alt_ids: Vec<Annotated<IdRef>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `subset` clauses.
    pub subsets: Vec<Annotated<IdRef>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `synonym` clauses of every scope, including BROAD and RELATED ones that are
    /// never matched.
    pub synonyms: Vec<Annotated<Synonym>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `xref` clauses.
    pub xrefs: Vec<Annotated<Xref>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `property_value` clauses.
    pub property_values: Vec<Annotated<PropertyValue>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `is_a` clauses.
    pub is_a: Vec<Annotated<IdRef>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `intersection_of` clauses.
    pub intersection_of: Vec<Annotated<Intersection>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `union_of` clauses.
    pub union_of: Vec<Annotated<IdRef>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `equivalent_to` clauses.
    pub equivalent_to: Vec<Annotated<IdRef>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `disjoint_from` clauses.
    pub disjoint_from: Vec<Annotated<IdRef>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `relationship` clauses.
    pub relationships: Vec<Annotated<Relationship>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `replaced_by` clauses.
    pub replaced_by: Vec<Annotated<IdRef>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// `consider` clauses.
    pub consider: Vec<Annotated<IdRef>>,
}

/// A clause line's payload plus its qualifiers and trailing `!` comment.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Annotated<T> {
    /// The clause payload. Its fields are flattened into the JSON object.
    #[serde(flatten)]
    pub value: T,
    /// The line's `{key="value"}` qualifiers, in order. Omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub qualifiers: Vec<Qualifier>,
    /// The trailing `!` comment, trimmed. Omitted from JSON when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// A `{key="value"}` qualifier on a clause line.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Qualifier {
    /// The qualifier key.
    pub key: String,
    /// The unquoted qualifier value.
    pub value: String,
}

/// A single-valued clause such as `namespace` or `comment`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Value {
    /// The clause value.
    pub value: String,
}

/// A reference to another identifier (`alt_id`, `is_a`, `subset`, ...).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct IdRef {
    /// The referenced identifier.
    pub id: String,
}

/// A `def` clause.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Definition {
    /// The definition text.
    pub text: String,
    /// The definition's cross-references, in order.
    pub xrefs: Vec<Xref>,
}

/// A `synonym` clause.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Synonym {
    /// The synonym text.
    pub text: String,
    /// The synonym's scope.
    pub scope: SynonymScope,
    /// The synonym type (e.g. `layperson`), serialized as `type`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// The synonym's cross-references, in order.
    pub xrefs: Vec<Xref>,
}

/// The scope of a synonym. Serializes as `"EXACT"`, `"NARROW"`, `"BROAD"` or `"RELATED"`.
///
/// Only EXACT synonyms are matched by [`search`](crate::search).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SynonymScope {
    /// `EXACT`
    Exact,
    /// `NARROW`
    Narrow,
    /// `BROAD`
    Broad,
    /// `RELATED`
    Related,
}

/// A cross-reference.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Xref {
    /// The cross-referenced identifier.
    pub id: String,
    /// The optional quoted description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A `property_value` clause.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PropertyValue {
    /// The property identifier.
    pub property: String,
    /// The target identifier, or the literal value.
    pub value: String,
    /// The literal's datatype (e.g. `xsd:string`); `None` when the value is an identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,
}

/// An `intersection_of` clause.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Intersection {
    /// The relation, when the clause has one (e.g. `part_of`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    /// The target identifier.
    pub id: String,
}

/// A `relationship` clause.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Relationship {
    /// The relation (e.g. `part_of`).
    pub relation: String,
    /// The target identifier.
    pub id: String,
}

fn xrefs(list: &ast::XrefList) -> Vec<Xref> {
    list.iter()
        .map(|x| Xref {
            id: x.id().to_string(),
            description: x.description().map(|d| d.as_str().to_string()),
        })
        .collect()
}

fn id_ref(id: &impl ToString) -> IdRef {
    IdRef { id: id.to_string() }
}

/// `fastobo` keeps the whitespace between an unquoted value and a trailing qualifier
/// list or comment, so it is trimmed here.
fn value(v: &impl ToString) -> Value {
    Value {
        value: v.to_string().trim_end().to_string(),
    }
}

impl Term {
    /// Converts a term frame. Returns `None` when the frame has no `name` clause.
    /// If there are several `name` clauses, the first is used.
    pub(crate) fn from_frame(frame: &TermFrame) -> Option<Term> {
        let name = frame.clauses().iter().find_map(|l| match l.as_inner() {
            TermClause::Name(n) => Some(n.as_str().trim_end().to_string()),
            _ => None,
        })?;
        let mut t = Term {
            id: frame.id().as_inner().to_string(),
            name,
            is_anonymous: None,
            builtin: None,
            namespace: None,
            comment: None,
            created_by: None,
            creation_date: None,
            def: None,
            alt_ids: Vec::new(),
            subsets: Vec::new(),
            synonyms: Vec::new(),
            xrefs: Vec::new(),
            property_values: Vec::new(),
            is_a: Vec::new(),
            intersection_of: Vec::new(),
            union_of: Vec::new(),
            equivalent_to: Vec::new(),
            disjoint_from: Vec::new(),
            relationships: Vec::new(),
            replaced_by: Vec::new(),
            consider: Vec::new(),
        };
        for line in frame.clauses() {
            match line.as_inner() {
                TermClause::IsAnonymous(b) => t.is_anonymous = Some(*b),
                TermClause::Builtin(b) => t.builtin = Some(*b),
                TermClause::Name(_) | TermClause::IsObsolete(_) => {}
                TermClause::Namespace(n) => t.namespace = Some(annotate(line, value(n))),
                TermClause::Comment(c) => t.comment = Some(annotate(line, value(&c.as_str()))),
                TermClause::CreatedBy(c) => t.created_by = Some(annotate(line, value(&c.as_str()))),
                TermClause::CreationDate(d) => t.creation_date = Some(annotate(line, value(d))),
                TermClause::Def(d) => {
                    t.def = Some(annotate(
                        line,
                        Definition {
                            text: d.text().as_str().to_string(),
                            xrefs: xrefs(d.xrefs()),
                        },
                    ))
                }
                TermClause::AltId(id) => t.alt_ids.push(annotate(line, id_ref(id))),
                TermClause::Subset(id) => t.subsets.push(annotate(line, id_ref(id))),
                TermClause::Synonym(s) => t.synonyms.push(annotate(
                    line,
                    Synonym {
                        text: s.description().as_str().to_string(),
                        scope: match s.scope() {
                            ast::SynonymScope::Exact => SynonymScope::Exact,
                            ast::SynonymScope::Narrow => SynonymScope::Narrow,
                            ast::SynonymScope::Broad => SynonymScope::Broad,
                            ast::SynonymScope::Related => SynonymScope::Related,
                        },
                        r#type: s.ty().map(|ty| ty.to_string()),
                        xrefs: xrefs(s.xrefs()),
                    },
                )),
                TermClause::Xref(x) => t.xrefs.push(annotate(
                    line,
                    Xref {
                        id: x.id().to_string(),
                        description: x.description().map(|d| d.as_str().to_string()),
                    },
                )),
                TermClause::PropertyValue(pv) => t.property_values.push(annotate(
                    line,
                    match pv.as_ref() {
                        ast::PropertyValue::Resource(r) => PropertyValue {
                            property: r.property().to_string(),
                            value: r.target().to_string(),
                            datatype: None,
                        },
                        ast::PropertyValue::Literal(l) => PropertyValue {
                            property: l.property().to_string(),
                            value: l.literal().as_str().to_string(),
                            datatype: Some(l.datatype().to_string()),
                        },
                    },
                )),
                TermClause::IsA(id) => t.is_a.push(annotate(line, id_ref(id))),
                TermClause::IntersectionOf(rel, id) => t.intersection_of.push(annotate(
                    line,
                    Intersection {
                        relation: rel.as_ref().map(|r| r.to_string()),
                        id: id.to_string(),
                    },
                )),
                TermClause::UnionOf(id) => t.union_of.push(annotate(line, id_ref(id))),
                TermClause::EquivalentTo(id) => t.equivalent_to.push(annotate(line, id_ref(id))),
                TermClause::DisjointFrom(id) => t.disjoint_from.push(annotate(line, id_ref(id))),
                TermClause::Relationship(rel, id) => t.relationships.push(annotate(
                    line,
                    Relationship {
                        relation: rel.to_string(),
                        id: id.to_string(),
                    },
                )),
                TermClause::ReplacedBy(id) => t.replaced_by.push(annotate(line, id_ref(id))),
                TermClause::Consider(id) => t.consider.push(annotate(line, id_ref(id))),
            }
        }
        Some(t)
    }
}

fn annotate<T, U>(line: &Line<T>, value: U) -> Annotated<U> {
    Annotated {
        value,
        qualifiers: line
            .qualifiers()
            .map(|qs| {
                qs.iter()
                    .map(|q| Qualifier {
                        key: q.key().to_string(),
                        value: q.value().as_str().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        comment: line.comment().map(|c| c.as_str().trim().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(src: &str) -> fastobo::ast::TermFrame {
        let doc = fastobo::from_str(format!("format-version: 1.2\n\n{src}")).unwrap();
        match doc.entities().first().unwrap() {
            fastobo::ast::EntityFrame::Term(t) => (**t).clone(),
            _ => panic!("not a term"),
        }
    }

    fn term(src: &str) -> Term {
        Term::from_frame(&frame(src)).expect("named term")
    }

    fn json(src: &str) -> String {
        serde_json::to_string(&term(src)).unwrap()
    }

    fn ids(v: &[Annotated<IdRef>]) -> Vec<&str> {
        v.iter().map(|a| a.value.id.as_str()).collect()
    }

    #[test]
    fn nameless_term_has_no_term() {
        assert!(Term::from_frame(&frame("[Term]\nid: HP:1\nis_a: HP:2\n")).is_none());
    }

    #[test]
    fn minimal_term_json() {
        assert_eq!(
            json("[Term]\nid: HP:0000001\nname: All\n"),
            r#"{"id":"HP:0000001","name":"All"}"#
        );
    }

    #[test]
    fn spec_synonym_and_parent_json() {
        let src = "[Term]\nid: HP:0000252\nname: Microcephaly\n\
                   synonym: \"Small head\" EXACT layperson []\n\
                   is_a: HP:0040195 ! Decreased head circumference\n";
        assert_eq!(
            json(src),
            r#"{"id":"HP:0000252","name":"Microcephaly","synonyms":[{"text":"Small head","scope":"EXACT","type":"layperson","xrefs":[]}],"is_a":[{"id":"HP:0040195","comment":"Decreased head circumference"}]}"#
        );
    }

    #[test]
    fn annotated_xref_json() {
        let t = term(
            "[Term]\nid: HP:1\nname: A\nxref: UMLS:C0025958 {source=\"MONDO\"} ! from MONDO\n",
        );
        assert_eq!(
            serde_json::to_string(&t.xrefs[0]).unwrap(),
            r#"{"id":"UMLS:C0025958","qualifiers":[{"key":"source","value":"MONDO"}],"comment":"from MONDO"}"#
        );
    }

    #[test]
    fn repeated_clauses_keep_file_order() {
        let t = term("[Term]\nid: HP:1\nname: A\nis_a: HP:0000001\nis_a: HP:0000002\n");
        assert_eq!(
            serde_json::to_value(&t).unwrap()["is_a"],
            serde_json::json!([{"id":"HP:0000001"},{"id":"HP:0000002"}])
        );
    }

    #[test]
    fn synonym_scopes_in_file_order() {
        let t = term(
            "[Term]\nid: HP:1\nname: A\nsynonym: \"a\" EXACT []\nsynonym: \"b\" NARROW []\n\
             synonym: \"c\" BROAD []\nsynonym: \"d\" RELATED []\n",
        );
        let v = serde_json::to_value(&t).unwrap();
        let scopes: Vec<&str> = v["synonyms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["scope"].as_str().unwrap())
            .collect();
        assert_eq!(scopes, ["EXACT", "NARROW", "BROAD", "RELATED"]);
        assert!(v["synonyms"][0].get("type").is_none());
    }

    #[test]
    fn every_clause_type_converts() {
        let t = term(
            "[Term]\nid: HP:0000118\nis_anonymous: false\nname: Phenotypic abnormality\n\
             namespace: human_phenotype\nalt_id: HP:0000999\n\
             def: \"A def.\" [PMID:1, HPO:x \"desc\"]\ncomment: Root.\nsubset: hp_slim\n\
             synonym: \"Pheno\" EXACT layperson [ORCID:1]\nxref: UMLS:C1 \"umls desc\"\nbuiltin: true\n\
             property_value: IAO:0000589 \"phenotypic abnormality\" xsd:string\n\
             property_value: RO:0002161 NCBITaxon:9606\nis_a: HP:0000001 ! All\n\
             intersection_of: HP:0000001\nintersection_of: part_of HP:0000002\n\
             union_of: HP:0000003\nunion_of: HP:0000004\nequivalent_to: HP:0000005\n\
             disjoint_from: HP:0000006\nrelationship: part_of HP:0000001\ncreated_by: peter\n\
             creation_date: 2008-02-27T02:20:00Z\nreplaced_by: HP:0000007\nconsider: HP:0000008\n",
        );
        assert_eq!(t.id, "HP:0000118");
        assert_eq!(t.name, "Phenotypic abnormality");
        assert_eq!(t.is_anonymous, Some(false));
        assert_eq!(t.builtin, Some(true));
        assert_eq!(t.namespace.as_ref().unwrap().value.value, "human_phenotype");
        assert_eq!(t.comment.as_ref().unwrap().value.value, "Root.");
        assert_eq!(t.created_by.as_ref().unwrap().value.value, "peter");
        assert_eq!(
            t.creation_date.as_ref().unwrap().value.value,
            "2008-02-27T02:20:00Z"
        );
        let def = &t.def.as_ref().unwrap().value;
        assert_eq!(def.text, "A def.");
        assert_eq!(
            def.xrefs,
            vec![
                Xref {
                    id: "PMID:1".into(),
                    description: None
                },
                Xref {
                    id: "HPO:x".into(),
                    description: Some("desc".into())
                }
            ]
        );
        assert_eq!(ids(&t.alt_ids), ["HP:0000999"]);
        assert_eq!(ids(&t.subsets), ["hp_slim"]);
        let syn = &t.synonyms[0].value;
        assert_eq!(syn.text, "Pheno");
        assert_eq!(syn.scope, SynonymScope::Exact);
        assert_eq!(syn.r#type.as_deref(), Some("layperson"));
        assert_eq!(
            syn.xrefs,
            vec![Xref {
                id: "ORCID:1".into(),
                description: None
            }]
        );
        assert_eq!(
            t.xrefs[0].value,
            Xref {
                id: "UMLS:C1".into(),
                description: Some("umls desc".into())
            }
        );
        assert_eq!(
            t.property_values
                .iter()
                .map(|p| p.value.clone())
                .collect::<Vec<_>>(),
            vec![
                PropertyValue {
                    property: "IAO:0000589".into(),
                    value: "phenotypic abnormality".into(),
                    datatype: Some("xsd:string".into())
                },
                PropertyValue {
                    property: "RO:0002161".into(),
                    value: "NCBITaxon:9606".into(),
                    datatype: None
                },
            ]
        );
        assert_eq!(ids(&t.is_a), ["HP:0000001"]);
        assert_eq!(t.is_a[0].comment.as_deref(), Some("All"));
        assert_eq!(
            t.intersection_of
                .iter()
                .map(|i| i.value.clone())
                .collect::<Vec<_>>(),
            vec![
                Intersection {
                    relation: None,
                    id: "HP:0000001".into()
                },
                Intersection {
                    relation: Some("part_of".into()),
                    id: "HP:0000002".into()
                },
            ]
        );
        assert_eq!(ids(&t.union_of), ["HP:0000003", "HP:0000004"]);
        assert_eq!(ids(&t.equivalent_to), ["HP:0000005"]);
        assert_eq!(ids(&t.disjoint_from), ["HP:0000006"]);
        assert_eq!(
            t.relationships[0].value,
            Relationship {
                relation: "part_of".into(),
                id: "HP:0000001".into()
            }
        );
        assert_eq!(ids(&t.replaced_by), ["HP:0000007"]);
        assert_eq!(ids(&t.consider), ["HP:0000008"]);

        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(
            v["namespace"],
            serde_json::json!({"value":"human_phenotype"})
        );
        assert_eq!(v["is_anonymous"], serde_json::json!(false));
        assert_eq!(
            v["def"]["xrefs"][1],
            serde_json::json!({"id":"HPO:x","description":"desc"})
        );
        assert_eq!(
            v["intersection_of"][0],
            serde_json::json!({"id":"HP:0000001"})
        );
        assert_eq!(
            v["property_values"][1],
            serde_json::json!({"property":"RO:0002161","value":"NCBITaxon:9606"})
        );
    }

    #[test]
    fn qualifiers_and_comments_are_carried() {
        let t = term(
            "[Term]\nid: HP:1\nname: A\nis_a: HP:2 {source=\"PMID:1\", note=\"x\"} ! Parent\n\
             synonym: \"S\" NARROW [] {source=\"PMID:2\"}\n",
        );
        assert_eq!(
            t.is_a[0].qualifiers,
            vec![
                Qualifier {
                    key: "source".into(),
                    value: "PMID:1".into()
                },
                Qualifier {
                    key: "note".into(),
                    value: "x".into()
                }
            ]
        );
        assert_eq!(t.is_a[0].comment.as_deref(), Some("Parent"));
        assert_eq!(
            t.synonyms[0].qualifiers,
            vec![Qualifier {
                key: "source".into(),
                value: "PMID:2".into()
            }]
        );
        assert_eq!(t.synonyms[0].comment, None);
    }

    #[test]
    fn unquoted_values_before_qualifiers_have_no_trailing_space() {
        let t = term(
            "[Term]\nid: HP:1\nname: All {source=\"x\"}\n\
             comment: Some text. {xref=\"HPO:probinson\"}\n\
             created_by: peter {source=\"y\"}\n\
             creation_date: 2008-02-27T02:20:00Z {source=\"z\"}\n",
        );
        assert_eq!(t.name, "All");
        let c = t.comment.as_ref().unwrap();
        assert_eq!(c.value.value, "Some text.");
        assert_eq!(
            c.qualifiers,
            vec![Qualifier {
                key: "xref".into(),
                value: "HPO:probinson".into()
            }]
        );
        assert_eq!(t.created_by.as_ref().unwrap().value.value, "peter");
        assert_eq!(
            t.creation_date.as_ref().unwrap().value.value,
            "2008-02-27T02:20:00Z"
        );
    }
}
