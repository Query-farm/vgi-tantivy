//! Table functions exposed by the tantivy worker, registered under `tantivy.main`.
//!
//! DuckDB table functions take *constant* arguments (no row columns, no
//! subqueries), so `docs_json` / `query` / `lang` are bind-time constants, passed
//! positionally (e.g. `bm25_search('[…]', 'cat')`). The corpus is therefore handed
//! in as a single JSON payload (see `bm25_search` for the contract), assembled in
//! SQL with `json_group_array`.

mod bm25_search;
mod supported_languages;

use std::sync::Arc;

use vgi::catalog::CatTable;
use vgi::Worker;

/// Register every table function on the worker.
pub fn register(worker: &mut Worker) {
    worker.register_table(bm25_search::Bm25Search);
    worker.register_table(supported_languages::SupportedLanguages);
}

/// A function-backed catalog table for the parameterless `supported_languages()`
/// table function, so it can also be used as `SELECT * FROM tantivy.main.supported_languages`
/// (no parentheses). A parameterless table function always returns the same rows,
/// so exposing it as a regular table is the idiomatic, friendlier surface (VGI311).
/// The backing scan function is auto-registered by `Worker::set_catalog`.
pub fn supported_languages_table() -> CatTable {
    let mut t = CatTable::with_function(
        "supported_languages",
        supported_languages::output_schema(),
        Arc::new(supported_languages::SupportedLanguages),
        Some(
            "The Snowball stemmer language ids this worker supports (valid `lang` values for \
             tokenize, stem, and the search analyzer)."
                .to_string(),
        ),
        Some(crate::search::SUPPORTED_LANGUAGES.len() as i64),
    );
    // `lang` (column 0) is the unique row identity — declare it the primary key
    // (VGI807) plus the table's NOT NULL/unique constraints (VGI806).
    t.primary_key = vec![vec![0]];
    t.not_null = vec![0];
    t.unique = vec![vec![0]];
    // Carry the same discovery tags + example queries as a well-documented
    // catalog object so the table lints clean on its own (VGI112/113/123/124/126/501).
    t.tags = vec![
        (
            "vgi.title".to_string(),
            "Supported Stemmer Languages".to_string(),
        ),
        (
            "vgi.doc_llm".to_string(),
            "The discovery table of every Snowball stemmer language id this worker supports. \
             One row per language; the single `lang` column is the valid value to pass as the \
             `lang` argument of tokenize(text, lang) and stem(word, lang). Query it to find out \
             which languages are available before stemming or tokenizing."
                .to_string(),
        ),
        (
            "vgi.doc_md".to_string(),
            "# supported_languages\n\nThe discovery table listing every Snowball stemmer language \
             id the worker supports. One row per language, with a single column `lang` (e.g. \
             `english`, `french`, `german`). These are exactly the valid `lang` values for \
             `tokenize(text, lang)`, `stem(word, lang)`, and the internal search analyzer. Use it \
             to enumerate the available languages."
                .to_string(),
        ),
        (
            "vgi.keywords".to_string(),
            crate::meta::keywords_json(&[
                "supported languages",
                "stemmer languages",
                "snowball languages",
                "available languages",
                "language list",
                "discovery",
                "tokenize languages",
                "stem languages",
            ]),
        ),
        ("domain".to_string(), "search".to_string()),
        ("category".to_string(), "discovery".to_string()),
        ("topic".to_string(), "stemmer-languages".to_string()),
        (
            "vgi.example_queries".to_string(),
            r#"[
  {
    "description": "List the first few supported Snowball stemmer languages.",
    "sql": "SELECT lang FROM tantivy.main.supported_languages ORDER BY lang LIMIT 5"
  },
  {
    "description": "Check whether a specific language is supported.",
    "sql": "SELECT count(*) > 0 AS supported FROM tantivy.main.supported_languages WHERE lang = 'english'"
  }
]"#
            .to_string(),
        ),
    ];
    t
}
