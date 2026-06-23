//! `bm25_search(docs_json, query) -> (doc_id BIGINT, score DOUBLE)` — rank a
//! corpus of documents against a full-text query by BM25 relevance, English
//! stemmer. One row per matching document, ordered by descending score (ties
//! broken by ascending doc_id for determinism).
//!
//! ## The `docs_json` contract
//! `docs_json` is a **constant** JSON payload describing the corpus (table
//! functions take constants, not subqueries). It is either:
//!   * a JSON array of strings — `["the cat sat","dogs bark"]` — where each
//!     document's `doc_id` is its 0-based array index; or
//!   * a JSON array of `{"id":<int>,"text":<string>}` objects, carrying an
//!     explicit `doc_id`.
//!
//! Assemble it in SQL from a real table with `json_group_array`:
//! ```sql
//! SELECT * FROM bm25_search(
//!   (SELECT json_group_array(text) FROM corpus), 'cat');
//! -- with explicit ids:
//! SELECT * FROM bm25_search(
//!   (SELECT json_group_array(json_object('id', id, 'text', body)) FROM corpus),
//!   'cat');
//! ```
//!
//! ## Ephemeral index
//! A brand-new tantivy index is built in a RAM directory for this one call and
//! dropped when it returns — nothing is persisted or shared across calls (see
//! `search.rs`). NULL/empty `docs_json` or NULL/blank `query` → no rows. Malformed
//! `docs_json` or an unparseable query is a clear error at bind.

use std::sync::Arc;

use arrow_array::builder::{Float64Builder, Int64Builder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use vgi::table_function::{TableFunction, TableProducer};
use vgi::{ArgSpec, BindParams, BindResponse, FunctionMetadata, ProcessParams};
use vgi_rpc::{OutputCollector, Result, RpcError};

use crate::search::{self, Hit};

/// The fixed stemmer language for corpus ranking. English by default; the full
/// language set is discoverable via `supported_languages()` and usable through
/// `tokenize`/`stem`.
const SEARCH_LANG: &str = "english";

/// Upper bound on the number of ranked hits returned.
const RESULT_LIMIT: usize = 10_000;

fn ve(e: impl std::fmt::Display) -> RpcError {
    RpcError::value_error(e.to_string())
}

pub struct Bm25Search;

fn output_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("doc_id", DataType::Int64, false),
        Field::new("score", DataType::Float64, false),
    ]))
}

impl TableFunction for Bm25Search {
    fn name(&self) -> &str {
        "bm25_search"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description:
                "BM25 full-text ranking of a JSON document corpus against a query, as (doc_id, score) rows"
                    .into(),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::const_arg(
                "docs_json",
                0,
                "varchar",
                "Corpus as a JSON array of strings or {id,text} objects",
            ),
            ArgSpec::const_arg("query", 1, "varchar", "Full-text query string"),
        ]
    }

    fn on_bind(&self, params: &BindParams) -> Result<BindResponse> {
        // Validate the payload + query eagerly at bind for a clear early error.
        if let (Some(docs), Some(query)) =
            (params.arguments.const_str(0), params.arguments.const_str(1))
        {
            let parsed = search::parse_docs(&docs).map_err(ve)?;
            // Surface a malformed-query error at bind time too.
            search::bm25_search(&parsed, &query, SEARCH_LANG, 1).map_err(ve)?;
        }
        Ok(BindResponse {
            output_schema: output_schema(),
            opaque_data: Vec::new(),
        })
    }

    fn producer(&self, params: &ProcessParams) -> Result<Box<dyn TableProducer>> {
        let hits = match (params.arguments.const_str(0), params.arguments.const_str(1)) {
            (Some(docs_json), Some(query)) => {
                let docs = search::parse_docs(&docs_json).map_err(ve)?;
                search::bm25_search(&docs, &query, SEARCH_LANG, RESULT_LIMIT).map_err(ve)?
            }
            // NULL docs_json or NULL query → no rows.
            _ => Vec::new(),
        };
        Ok(Box::new(Bm25Producer {
            schema: params.output_schema.clone(),
            hits,
            done: false,
        }))
    }
}

struct Bm25Producer {
    schema: SchemaRef,
    hits: Vec<Hit>,
    done: bool,
}

impl TableProducer for Bm25Producer {
    fn next_batch(&mut self, _out: &mut OutputCollector) -> Result<Option<RecordBatch>> {
        if self.done {
            return Ok(None);
        }
        self.done = true;

        let mut ids = Int64Builder::new();
        let mut scores = Float64Builder::new();
        for h in &self.hits {
            ids.append_value(h.doc_id);
            scores.append_value(h.score);
        }
        let cols: Vec<ArrayRef> = vec![Arc::new(ids.finish()), Arc::new(scores.finish())];
        Ok(Some(
            RecordBatch::try_new(self.schema.clone(), cols)
                .map_err(|e| RpcError::runtime_error(e.to_string()))?,
        ))
    }
}
