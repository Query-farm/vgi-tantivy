//! The `tantivy` VGI worker.
//!
//! A standalone binary that DuckDB launches and talks to over Apache Arrow IPC
//! (`ATTACH 'tantivy' (TYPE vgi, LOCATION '…')`). It provides full-text search
//! (BM25 ranking) plus tokenization and Snowball stemming, powered by the
//! [tantivy](https://github.com/quickwit-oss/tantivy) search engine, under the
//! catalog `tantivy`, schema `main`:
//!
//! ```sql
//! ATTACH 'tantivy' (TYPE vgi, LOCATION './target/release/tantivy-worker');
//! SET search_path = 'tantivy.main';
//!
//! -- Rank a JSON corpus by BM25 relevance (table function).
//! SELECT * FROM bm25_search('["the cat sat","dogs bark","stock crash"]', 'cat');
//! --   doc_id | score
//! --        0 | …
//!
//! SELECT UNNEST(tokenize('Running quickly'));     -- ['running','quickly']
//! SELECT stem('running', 'english');              -- → 'run'
//! SELECT bm25_score('the cat sat', 'cat');        -- ad-hoc single-doc score
//! SELECT * FROM supported_languages();            -- stemmer languages
//! SELECT tantivy_version();
//! ```
//!
//! Pure search/analysis logic (no Arrow) lives in `search.rs`; the `scalar/` and
//! `table/` modules are thin Arrow adapters over it. Every index is built in a RAM
//! directory **per call** and never persisted (see `search.rs` for the
//! ephemeral-index semantics).

mod arrow_io;
mod meta;
mod scalar;
mod search;
mod table;

use vgi::catalog::{CatSchema, CatalogModel};
use vgi::Worker;

/// Worker version string, surfaced by `tantivy_version()` alongside the tantivy
/// engine version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Catalog + schema metadata (description, provenance, support) surfaced to
/// DuckDB and the `vgi-lint` metadata-quality linter. The function objects
/// themselves are served from the registered scalars/tables; this only adds the
/// catalog/schema-level comments and tags.
fn catalog_metadata(name: &str) -> CatalogModel {
    CatalogModel {
        name: name.to_string(),
        comment: Some(
            "Full-text search (BM25 ranking), tokenization, and Snowball stemming, powered by the \
             tantivy search engine."
                .to_string(),
        ),
        tags: vec![
            (
                "vgi.title".to_string(),
                "Full-Text Search & Text Analysis".to_string(),
            ),
            (
                "vgi.keywords".to_string(),
                "full-text search, BM25, relevance ranking, tantivy, tokenize, tokenization, \
                 stemming, snowball stemmer, text analysis, search, scoring, information retrieval"
                    .to_string(),
            ),
            (
                "vgi.description_llm".to_string(),
                "Rank a corpus of documents against a full-text query by BM25 relevance, score a \
                 single document ad-hoc, tokenize text into terms (optionally with a language \
                 stemmer), and Snowball-stem individual words. Use for full-text search, relevance \
                 ranking, and text analysis (tokenization/stemming) in SQL. Backed by the tantivy \
                 search engine with per-call in-RAM indexes (nothing is persisted)."
                    .to_string(),
            ),
            (
                "vgi.description_md".to_string(),
                "# tantivy\n\nFull-text search (BM25 ranking) plus tokenization and Snowball \
                 stemming over Apache Arrow, powered by the \
                 [tantivy](https://github.com/quickwit-oss/tantivy) search engine.\n\nScalars: \
                 `tokenize` (1- and 2-arg), `stem`, `bm25_score`, `tantivy_version`. Tables: \
                 `bm25_search`, `supported_languages`. Every index is built in a RAM directory per \
                 call and never persisted."
                    .to_string(),
            ),
            ("vgi.author".to_string(), "Query.Farm".to_string()),
            (
                "vgi.copyright".to_string(),
                "Copyright 2026 Query Farm LLC - https://query.farm".to_string(),
            ),
            ("vgi.license".to_string(), "MIT".to_string()),
            (
                "vgi.support_contact".to_string(),
                "https://github.com/Query-farm/vgi-tantivy/issues".to_string(),
            ),
            (
                "vgi.support_policy_url".to_string(),
                "https://github.com/Query-farm/vgi-tantivy/blob/main/README.md".to_string(),
            ),
        ],
        source_url: Some("https://github.com/Query-farm/vgi-tantivy".to_string()),
        schemas: vec![CatSchema {
            name: "main".to_string(),
            comment: Some(
                "Full-text search, relevance scoring, and text-analysis functions.".to_string(),
            ),
            tags: vec![
                ("vgi.title".to_string(), "Tantivy — main".to_string()),
                (
                    "vgi.keywords".to_string(),
                    "full-text search, BM25, bm25_search, bm25_score, tokenize, stem, stemming, \
                     supported_languages, tantivy_version, relevance ranking, text analysis, \
                     information retrieval"
                        .to_string(),
                ),
                // VGI123 classifying tags (bare keys: domain/category/topic) for faceting.
                ("domain".to_string(), "search".to_string()),
                ("category".to_string(), "full-text-search".to_string()),
                ("topic".to_string(), "bm25-ranking".to_string()),
                (
                    "vgi.source_url".to_string(),
                    "https://github.com/Query-farm/vgi-tantivy/blob/main/crates/tantivy-worker/src/main.rs"
                        .to_string(),
                ),
                (
                    "vgi.description_llm".to_string(),
                    "Full-text search and text-analysis functions: rank a JSON document corpus by \
                     BM25 relevance, score a single document ad-hoc, tokenize text (optionally \
                     with a language stemmer), Snowball-stem words, and list supported stemmer \
                     languages."
                        .to_string(),
                ),
                (
                    "vgi.description_md".to_string(),
                    "Full-text search (BM25), relevance scoring, tokenization, and Snowball \
                     stemming over Apache Arrow."
                        .to_string(),
                ),
                // VGI506 representative example queries for the schema.
                (
                    "vgi.example_queries".to_string(),
                    "SELECT * FROM tantivy.main.bm25_search('[\"the cat sat\",\"dogs bark\",\"stock crash\"]', 'cat');\n\
                     SELECT tantivy.main.bm25_score('the cat sat on the mat', 'cat');\n\
                     SELECT tantivy.main.tokenize('Running quickly, CATS!');\n\
                     SELECT tantivy.main.tokenize('Running quickly', 'english');\n\
                     SELECT tantivy.main.stem('running', 'english');\n\
                     SELECT * FROM tantivy.main.supported_languages();\n\
                     SELECT tantivy.main.tantivy_version();"
                        .to_string(),
                ),
            ],
            views: Vec::new(),
            macros: Vec::new(),
            tables: Vec::new(),
        }],
        ..Default::default()
    }
}

fn main() {
    // Logs MUST go to stderr — stdout is the Arrow-IPC channel.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().filter_or("VGI_LOG", "info"))
        .format_timestamp_millis()
        .try_init();

    // The catalog name DuckDB sees in `ATTACH 'tantivy' (TYPE vgi, …)`. Default to
    // `tantivy`, but honor an explicit override so a test harness can rename it.
    if std::env::var_os("VGI_WORKER_CATALOG_NAME").is_none() {
        std::env::set_var("VGI_WORKER_CATALOG_NAME", "tantivy");
    }

    let catalog_name =
        std::env::var("VGI_WORKER_CATALOG_NAME").unwrap_or_else(|_| "tantivy".to_string());

    let mut worker = Worker::new();
    scalar::register(&mut worker);
    table::register(&mut worker);
    worker.set_catalog(catalog_metadata(&catalog_name));
    worker.run();
}
