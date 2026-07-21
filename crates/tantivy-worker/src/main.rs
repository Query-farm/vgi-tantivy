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
//! ```
//!
//! The underlying tantivy engine version is published as catalog metadata
//! (`vgi_catalogs().implementation_version` and the `engine_version` tag), not as
//! a query-consuming scalar.
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

/// Worker build version, published as the catalog `implementation_version` so an
/// agent reads it from `vgi_catalogs()` without spending a query (VGI328).
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
                meta::keywords_json(&[
                    "full-text search",
                    "BM25",
                    "relevance ranking",
                    "tantivy",
                    "tokenize",
                    "tokenization",
                    "stemming",
                    "snowball stemmer",
                    "text analysis",
                    "search",
                    "scoring",
                    "information retrieval",
                ]),
            ),
            (
                "vgi.doc_llm".to_string(),
                "Rank a corpus of documents against a full-text query by BM25 relevance, score a \
                 single document ad-hoc, tokenize text into terms (optionally with a language \
                 stemmer), and Snowball-stem individual words. Use for full-text search, relevance \
                 ranking, and text analysis (tokenization/stemming) in SQL. Backed by the tantivy \
                 search engine with per-call in-RAM indexes (nothing is persisted)."
                    .to_string(),
            ),
            (
                "vgi.doc_md".to_string(),
                "# tantivy — Full-Text Search & Text Analysis in SQL\n\n\
                 ![tantivy logo](https://tantivy-search.github.io/logo/tantivy-logo.png)\n\n\
                 Add **BM25 full-text search**, relevance ranking, and query-time text analysis \
                 (tokenization and Snowball stemming) directly to DuckDB SQL — no separate search \
                 service, index server, or ETL pipeline required. The `tantivy` worker is a \
                 standalone VGI worker that DuckDB attaches over Apache Arrow, exposing lexical \
                 (keyword) search and text-normalization functions you can call inline from any \
                 query.\n\n\
                 ## How it works\n\n\
                 Every function is powered by \
                 [tantivy](https://github.com/quickwit-oss/tantivy), the fast full-text search \
                 engine written in Rust (the library behind Quickwit), documented at \
                 [docs.rs/tantivy](https://docs.rs/tantivy/latest/tantivy/). For each \
                 `bm25_search` / `bm25_score` call the worker builds a fresh in-RAM tantivy index, \
                 registers the appropriate per-language analyzer (simple tokenizer + lowercasing + \
                 a Snowball stemmer), indexes your documents, runs the query through tantivy's \
                 query parser and top-docs collector, and drops the index when the call returns. \
                 Nothing is persisted or shared between calls, so results are fully deterministic \
                 and there is no external state to manage. Relevance is computed with the \
                 industry-standard [Okapi BM25](https://en.wikipedia.org/wiki/Okapi_BM25) ranking \
                 model, and stemming uses the [Snowball](https://snowballstem.org/) algorithms.\n\n\
                 ## Key concepts\n\n\
                 Corpora are passed **inline as constant JSON** rather than as table references, \
                 because the ranking function binds its corpus at query-bind time: assemble a text \
                 column into a JSON payload with `json_group_array` and hand it in. Relevance is \
                 lexical (keyword/BM25), not semantic — there is no embedding model or vector \
                 store. Text analysis is language-aware through Snowball stemming, so word variants \
                 collapse to a shared root; the set of stemmer languages is discoverable at \
                 runtime.\n\n\
                 ## Who it's for\n\n\
                 Reach for this worker whenever you want lightweight, embedded lexical search and \
                 relevance ranking inside an analytical SQL workflow — log triage, document \
                 shortlisting, fuzzy keyword matching, or normalizing text before joins and \
                 grouping — without standing up Elasticsearch, OpenSearch, or a dedicated indexing \
                 service. Every index is built in RAM **per call** and dropped immediately, and \
                 corpora are passed inline as constant JSON, so queries run with no external \
                 state."
                    .to_string(),
            ),
            // VGI328: the underlying tantivy engine version, published as catalog
            // metadata (an agent reads it from `vgi_catalogs()` without spending a
            // query, and it can't drift from the running build) instead of as a
            // parameterless `tantivy_version()` scalar. e.g.
            // `tantivy v0.24.2, index_format v7`.
            (
                "engine_version".to_string(),
                crate::search::tantivy_version(),
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
            // VGI152: analyst task suite for `vgi-lint simulate`. Each task is
            // self-contained and deterministic — corpora are passed inline as
            // constant JSON, so no external table or attached data is required.
            (
                "vgi.agent_test_tasks".to_string(),
                r#"[
  {
    "name": "rank corpus by relevance",
    "prompt": "Given exactly these three documents, in this order — 'the cat sat', 'dogs bark', and 'a cat and a dog' — rank them by BM25 relevance to the query 'cat'. Return every matching row as (doc_id, score), best score first, exactly as the worker produces them.",
    "reference_sql": "SELECT doc_id, score FROM tantivy.main.bm25_search('[\"the cat sat\",\"dogs bark\",\"a cat and a dog\"]', 'cat') ORDER BY score DESC, doc_id"
  },
  {
    "name": "rank corpus with explicit ids",
    "prompt": "Rank this corpus, whose documents carry explicit ids, by BM25 relevance to the query 'cat': the document with id 10 is 'the cat sat', id 20 is 'stock crash', and id 30 is 'the cat and the dog'. Return the matching rows as (doc_id, score).",
    "reference_sql": "SELECT doc_id, score FROM tantivy.main.bm25_search('[{\"id\":10,\"text\":\"the cat sat\"},{\"id\":20,\"text\":\"stock crash\"},{\"id\":30,\"text\":\"the cat and the dog\"}]', 'cat') ORDER BY score DESC, doc_id"
  },
  {
    "name": "score one document",
    "prompt": "Run the worker to compute the ad-hoc BM25 relevance score of the single document 'the cat sat on the mat' against the query 'cat'. Execute the function against these exact inputs and return the numeric score it outputs — do not estimate it or reuse a value from documentation.",
    "reference_sql": "SELECT tantivy.main.bm25_score('the cat sat on the mat', 'cat') AS score"
  },
  {
    "name": "tokenize without stemming",
    "prompt": "Use this worker to split the text 'Running quickly, CATS!' into lowercased word tokens WITHOUT any stemming (the plain 1-argument tokenizer). Run the function and return the resulting array of tokens exactly as produced.",
    "reference_sql": "SELECT tantivy.main.tokenize('Running quickly, CATS!') AS tokens"
  },
  {
    "name": "tokenize with english stemming",
    "prompt": "Use this worker to tokenize the text 'Running quickly' and additionally apply the English Snowball stemmer to each token (the 2-argument tokenizer). Return the resulting array of tokens exactly as produced.",
    "reference_sql": "SELECT tantivy.main.tokenize('Running quickly', 'english') AS tokens"
  },
  {
    "name": "list supported languages",
    "prompt": "List every Snowball stemmer language this worker supports, ordered alphabetically. Return the full list of language ids.",
    "reference_sql": "SELECT lang FROM tantivy.main.supported_languages() ORDER BY lang"
  },
  {
    "name": "stem a single word",
    "prompt": "Using this worker's single-word stemming function, reduce the individual English word 'running' to its Snowball root. Return exactly one row holding a single scalar text value — the stemmed root word itself (a plain string, not a list or array).",
    "reference_sql": "SELECT tantivy.main.stem('running', 'english') AS root",
    "ignore_column_names": true
  }
]"#
                .to_string(),
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
                    meta::keywords_json(&[
                        "full-text search",
                        "BM25",
                        "bm25_search",
                        "bm25_score",
                        "tokenize",
                        "stem",
                        "stemming",
                        "supported_languages",
                        "tantivy_version",
                        "relevance ranking",
                        "text analysis",
                        "information retrieval",
                    ]),
                ),
                // VGI123 classifying tags (bare keys: domain/category/topic) for faceting.
                ("domain".to_string(), "search".to_string()),
                ("category".to_string(), "full-text-search".to_string()),
                ("topic".to_string(), "bm25-ranking".to_string()),
                // VGI413: ordered category registry; every function/table carries a
                // matching `vgi.category` tag naming one of these.
                (
                    "vgi.categories".to_string(),
                    r#"[
  {"name": "Search & Ranking", "description": "BM25 full-text relevance ranking of a document corpus and ad-hoc single-document scoring."},
  {"name": "Text Analysis", "description": "Language-aware tokenization and Snowball stemming primitives for normalizing text."},
  {"name": "Discovery", "description": "Introspection helpers: the supported stemmer languages and the underlying engine version."}
]"#
                    .to_string(),
                ),
                (
                    "vgi.doc_llm".to_string(),
                    "Full-text search and text-analysis functions: rank a JSON document corpus by \
                     BM25 relevance, score a single document ad-hoc, tokenize text (optionally \
                     with a language stemmer), Snowball-stem words, and list supported stemmer \
                     languages."
                        .to_string(),
                ),
                (
                    "vgi.doc_md".to_string(),
                    "## tantivy.main\n\nThe single schema of the `tantivy` worker, grouping its \
                     full-text search, relevance-scoring, and text-analysis capabilities into \
                     three areas.\n\n\
                     ### What lives here\n\n\
                     **Search & ranking** covers BM25 relevance over a document corpus that you \
                     pass inline as a constant JSON payload, plus an ad-hoc single-document \
                     scorer. **Text analysis** provides language-aware tokenization and Snowball \
                     stemming for normalizing text before matching, grouping, or building search \
                     keys. **Discovery** lets you introspect which stemmer languages are available \
                     and which engine version is in use.\n\n\
                     ### How it behaves\n\n\
                     Every function is deterministic given its inputs and needs no persisted \
                     state — indexes are built in a RAM directory per call and dropped when the \
                     call returns."
                        .to_string(),
                ),
                // VGI506/VGI515 representative example queries for the schema, as a
                // JSON list of {description, sql} so every example carries a
                // human-readable description (the native newline-joined form drops
                // descriptions). Each is projection/filter/aggregation, not a bare
                // SELECT * (VGI514).
                (
                    "vgi.example_queries".to_string(),
                    r#"[
  {"description":"Rank a small JSON corpus by BM25 relevance to a query, best score first.","sql":"SELECT doc_id, score FROM tantivy.main.bm25_search('[\"the cat sat\",\"dogs bark\",\"stock crash\"]', 'cat') ORDER BY score DESC, doc_id"},
  {"description":"Compute the ad-hoc BM25 score of a single document against a query.","sql":"SELECT tantivy.main.bm25_score('the cat sat on the mat', 'cat') AS score"},
  {"description":"Tokenize text into lowercased word tokens (no stemming).","sql":"SELECT tantivy.main.tokenize('Running quickly, CATS!') AS tokens"},
  {"description":"Tokenize and Snowball-stem text for a language.","sql":"SELECT tantivy.main.tokenize('Running quickly', 'english') AS tokens"},
  {"description":"Snowball-stem a single word to its root for a language.","sql":"SELECT tantivy.main.stem('running', 'english') AS root"},
  {"description":"List the first few supported Snowball stemmer languages, alphabetically.","sql":"SELECT lang FROM tantivy.main.supported_languages ORDER BY lang LIMIT 5"}
]"#
                        .to_string(),
                ),
            ],
            views: Vec::new(),
            macros: Vec::new(),
            // Expose the parameterless `supported_languages()` table function as a
            // regular table so it is usable as `SELECT * FROM tantivy.main.supported_languages`
            // (no parentheses) — VGI311.
            tables: vec![table::supported_languages_table()],
        }],
        // VGI328: publish the worker build version as catalog metadata, readable
        // from `vgi_catalogs().implementation_version` without spending a query.
        implementation_version: Some(version().to_string()),
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
