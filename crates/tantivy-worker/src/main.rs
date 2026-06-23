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
mod scalar;
mod search;
mod table;

use vgi::Worker;

/// Worker version string, surfaced by `tantivy_version()` alongside the tantivy
/// engine version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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

    let mut worker = Worker::new();
    scalar::register(&mut worker);
    table::register(&mut worker);
    worker.run();
}
