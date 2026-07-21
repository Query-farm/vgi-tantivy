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

/// Build native `FunctionExample`s from the same `(description, sql)` pairs used
/// for the `vgi.example_queries` tag. Sharing one pair list keeps the native
/// carrier and the tag byte-identical: the linter dedupes examples by SQL and
/// takes the (preserved) description from the tag, so the description-dropping
/// `duckdb_functions().examples` copy is not double-counted (VGI515).
pub fn examples_from(pairs: &[(&str, &str)]) -> Vec<vgi::FunctionExample> {
    pairs
        .iter()
        .map(|(desc, sql)| vgi::FunctionExample {
            sql: (*sql).to_string(),
            description: (*desc).to_string(),
            expected_output: None,
        })
        .collect()
}

/// Build a `vgi.example_queries` tag from `(description, sql)` pairs as the
/// described-example JSON list `[{"description":…,"sql":…}]` the linter requires
/// (VGI515/VGI503). The native `FunctionExample` carrier surfaced via
/// `duckdb_functions().examples` drops the per-example description, so every
/// function also publishes its examples through this tag, which preserves them.
/// For arity-overloaded functions (e.g. `tokenize`) both overloads must publish
/// the SAME aggregated pair list so the merged catalog view is consistent.
pub fn example_queries_tag(examples: &[(&str, &str)]) -> (String, String) {
    let list: Vec<serde_json::Value> = examples
        .iter()
        .map(|(desc, sql)| serde_json::json!({"description": desc, "sql": sql}))
        .collect();
    (
        "vgi.example_queries".to_string(),
        serde_json::to_string(&list).expect("serializing example list to JSON never fails"),
    )
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
