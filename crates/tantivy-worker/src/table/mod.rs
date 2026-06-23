//! Table functions exposed by the tantivy worker, registered under `tantivy.main`.
//!
//! DuckDB table functions take *constant* arguments (no row columns, no
//! subqueries), so `docs_json` / `query` / `lang` are bind-time constants, passed
//! positionally (e.g. `bm25_search('[…]', 'cat')`). The corpus is therefore handed
//! in as a single JSON payload (see `bm25_search` for the contract), assembled in
//! SQL with `json_group_array`.

mod bm25_search;
mod supported_languages;

use vgi::Worker;

/// Register every table function on the worker.
pub fn register(worker: &mut Worker) {
    worker.register_table(bm25_search::Bm25Search);
    worker.register_table(supported_languages::SupportedLanguages);
}
