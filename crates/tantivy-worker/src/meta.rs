//! Shared helpers for the per-object discovery/description metadata that the
//! `vgi-lint` strict profile expects on **every** function and table.
//!
//! Each function/table surfaces these in its `FunctionMetadata.tags`:
//! - `vgi.title` (VGI124)        — human-friendly display name
//! - `vgi.doc_llm` (VGI112)      — Markdown narrative aimed at LLMs/agents
//! - `vgi.doc_md` (VGI113)       — Markdown narrative for human docs
//! - `vgi.keywords` (VGI126)        — JSON array of search terms/synonyms
//!
//! `vgi.source_url` (VGI139) is intentionally NOT set per object — it belongs
//! only on the catalog object, which carries the repo URL.
//!
//! `keywords_json(&[…])` serializes a keyword list as a JSON array of strings,
//! the form `vgi.keywords` requires (VGI138).

/// Serialize a keyword list as a JSON array of strings, e.g.
/// `keywords_json(&["bm25", "search"])` → `["bm25","search"]`. This is the form
/// `vgi.keywords` must take (VGI138); a comma-separated string is rejected.
pub fn keywords_json(keywords: &[&str]) -> String {
    serde_json::to_string(keywords).expect("serializing &str slice to JSON never fails")
}

/// Build the five standard per-object discovery/description tags.
///
/// `keywords` is a slice of search terms/synonyms, serialized to a JSON array of
/// strings (VGI138). `category` names the schema `vgi.categories` entry this
/// object belongs to (VGI413); it is surfaced as `vgi.category`. `vgi.source_url`
/// is deliberately omitted (VGI139): it lives only on the catalog object.
pub fn object_tags(
    title: &str,
    doc_llm: &str,
    doc_md: &str,
    keywords: &[&str],
    category: &str,
) -> Vec<(String, String)> {
    vec![
        ("vgi.title".to_string(), title.to_string()),
        ("vgi.doc_llm".to_string(), doc_llm.to_string()),
        ("vgi.doc_md".to_string(), doc_md.to_string()),
        ("vgi.keywords".to_string(), keywords_json(keywords)),
        ("vgi.category".to_string(), category.to_string()),
    ]
}
