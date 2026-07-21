//! Scalar functions exposed by the tantivy worker, registered under
//! `tantivy.main`.
//!
//! All scalars are positional-only (DuckDB scalar functions take no named args).
//! Every per-row value is read from a column; `lang`/`query` arguments are
//! likewise per-row VARCHAR columns (constant-folded by DuckDB when a literal is
//! passed), so a single call can mix languages/queries across rows.
//!
//! These are per-row **text analysis** primitives that need no corpus index
//! (`tokenize`, `stem`), plus the ad-hoc single-document scorer (`bm25_score`,
//! which builds a throwaway 1-doc index per row). Ranked corpus search lives in
//! the `table` module (`bm25_search`). The tantivy engine version is published as
//! catalog metadata (`implementation_version` and the `engine_version` tag), not
//! as a query-consuming scalar (VGI328).

mod analyze;
mod score;

use vgi::Worker;

/// Register every scalar function on the worker.
pub fn register(worker: &mut Worker) {
    worker.register_scalar(analyze::Tokenize);
    worker.register_scalar(analyze::TokenizeLang);
    worker.register_scalar(analyze::Stem);
    worker.register_scalar(score::Bm25Score);
}
