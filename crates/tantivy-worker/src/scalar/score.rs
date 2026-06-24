//! `bm25_score(doc_text, query) -> DOUBLE` — score a single document against a
//! query, English stemmer.
//!
//! This builds a throwaway **1-document** index per row (an ad-hoc relevance
//! probe). Because BM25 statistics depend on the whole corpus, scores from this
//! function are *not* comparable across documents/calls — for ranking a real
//! corpus use the `bm25_search` table function. A non-matching document scores
//! `0.0`; NULL doc/query → NULL.

use std::sync::Arc;

use arrow_array::builder::Float64Builder;
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::DataType;
use vgi::{
    ArgSpec, BindParams, BindResponse, FunctionExample, FunctionMetadata, ProcessParams,
    ScalarFunction,
};
use vgi_rpc::{Result, RpcError};

use crate::arrow_io::text_str;
use crate::search;

fn ve(e: impl std::fmt::Display) -> RpcError {
    RpcError::value_error(e.to_string())
}

/// The fixed stemmer language for the ad-hoc scorer. English is the default; the
/// corpus-ranking `bm25_search` table function exposes the full language choice.
const SCORE_LANG: &str = "english";

pub struct Bm25Score;

impl ScalarFunction for Bm25Score {
    fn name(&self) -> &str {
        "bm25_score"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description:
                "Ad-hoc BM25 score of a single document against a query (1-doc index; 0.0 if no match)"
                    .into(),
            return_type: Some(DataType::Float64),
            examples: vec![FunctionExample {
                sql: "SELECT tantivy.main.bm25_score('the cat sat on the mat', 'cat');".into(),
                description: "Ad-hoc BM25 relevance of a single document against a query \
                              (> 0.0 when it matches, 0.0 otherwise)."
                    .into(),
                expected_output: None,
            }],
            tags: crate::meta::object_tags(
                "BM25 Single-Document Score",
                "Compute an ad-hoc BM25 relevance score for one document against a query string, \
                 using the English stemmer and a throwaway 1-document index. Returns 0.0 when the \
                 document does not match the query. Because BM25 statistics depend on the whole \
                 corpus, scores are NOT comparable across rows — use bm25_search to rank a real \
                 corpus. NULL document or query → NULL.",
                "Ad-hoc BM25 score of a single document against a query, e.g. \
                 `bm25_score('the cat sat on the mat', 'cat')` (> 0.0 on match, 0.0 otherwise).",
                "bm25, bm25 score, relevance score, single document, ad-hoc score, full-text \
                 match, scoring, ranking probe, query match",
                "scalar/score.rs",
            ),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::any_column("doc_text", 0, "Document text (VARCHAR)"),
            ArgSpec::any_column("query", 1, "Query string (VARCHAR)"),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Float64))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let doc_col = batch.column(0);
        let query_col = batch.column(1);
        let rows = batch.num_rows();
        let mut out = Float64Builder::new();
        for i in 0..rows {
            match (text_str(doc_col, i)?, text_str(query_col, i)?) {
                (Some(doc), Some(query)) => {
                    out.append_value(search::bm25_score(doc, query, SCORE_LANG).map_err(ve)?)
                }
                // NULL doc or NULL query → NULL.
                _ => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_io::test_support::{bound_type, run_scalar_on, str_args, text_batch};
    use arrow_array::cast::AsArray;
    use arrow_array::types::Float64Type;
    use arrow_array::Array;

    fn score(doc: &str, query: &str) -> f64 {
        let batch = text_batch(&[&[Some(doc)], &[Some(query)]]);
        let out = run_scalar_on(&Bm25Score, batch, str_args(&[Some(doc), Some(query)])).unwrap();
        out.as_primitive::<Float64Type>().value(0)
    }

    #[test]
    fn binds_double() {
        assert_eq!(bound_type(&Bm25Score), DataType::Float64);
    }

    #[test]
    fn relevant_beats_irrelevant() {
        let relevant = score("the cat sat on the mat", "cat");
        let irrelevant = score("the stock market crashed", "cat");
        assert!(relevant > 0.0);
        assert_eq!(irrelevant, 0.0);
        assert!(relevant > irrelevant);
    }

    #[test]
    fn null_inputs_yield_null() {
        let batch = text_batch(&[&[None, Some("x")], &[Some("cat"), None]]);
        let out = run_scalar_on(&Bm25Score, batch, str_args(&[None, Some("cat")])).unwrap();
        assert!(out.is_null(0));
        assert!(out.is_null(1));
    }
}
