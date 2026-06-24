//! Per-row text analysis scalars (no index needed):
//! - `tokenize(text) -> VARCHAR[]` — default tokenizer (unicode words, lowercased).
//! - `tokenize(text, lang) -> VARCHAR[]` — language stemming variant.
//! - `stem(word, lang) -> VARCHAR` — Snowball-stem a single word.
//!
//! `lang` is a per-row VARCHAR column. NULL text → NULL output; an unknown
//! language is a clear error (the caller named it).

use std::sync::Arc;

use arrow_array::builder::StringBuilder;
use vgi::{
    ArgSpec, BindParams, BindResponse, FunctionExample, FunctionMetadata, ProcessParams,
    ScalarFunction,
};
use vgi_rpc::{Result, RpcError};

use crate::arrow_io::{
    append_list_null, append_list_row, finish_list, list_builder, list_varchar_type, text_str,
};
use crate::search;

fn ve(e: impl std::fmt::Display) -> RpcError {
    RpcError::value_error(e.to_string())
}

/// `tokenize(text) -> VARCHAR[]` — default (non-stemming) tokenizer.
pub struct Tokenize;

impl ScalarFunction for Tokenize {
    fn name(&self) -> &str {
        "tokenize"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description:
                "Tokenize text with the default tokenizer (unicode words, lowercased) as VARCHAR[]"
                    .into(),
            return_type: Some(list_varchar_type()),
            examples: vec![FunctionExample {
                sql: "SELECT tantivy.main.tokenize('Running quickly, CATS!');".into(),
                description: "Tokenize text into lowercased unicode word tokens \
                              (['running','quickly','cats'])."
                    .into(),
                expected_output: None,
            }],
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![ArgSpec::any_column("text", 0, "Text to tokenize (VARCHAR)")]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(list_varchar_type()))
    }

    fn process(
        &self,
        params: &ProcessParams,
        batch: &arrow_array::RecordBatch,
    ) -> Result<arrow_array::RecordBatch> {
        let col = batch.column(0);
        let rows = batch.num_rows();
        let mut builder = list_builder();
        for i in 0..rows {
            match text_str(col, i)? {
                None => append_list_null(&mut builder),
                Some(text) => {
                    let toks = search::tokenize_default(text).map_err(ve)?;
                    append_list_row(&mut builder, &toks);
                }
            }
        }
        let out = finish_list(builder);
        arrow_array::RecordBatch::try_new(params.output_schema.clone(), vec![out])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `tokenize(text, lang) -> VARCHAR[]` — language-stemming tokenizer variant.
pub struct TokenizeLang;

impl ScalarFunction for TokenizeLang {
    fn name(&self) -> &str {
        "tokenize"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description:
                "Tokenize text with the given language's Snowball-stemming tokenizer, as VARCHAR[]"
                    .into(),
            return_type: Some(list_varchar_type()),
            examples: vec![FunctionExample {
                sql: "SELECT tantivy.main.tokenize('Running quickly', 'english');".into(),
                description: "Tokenize and Snowball-stem text for a language (['run','quickli'])."
                    .into(),
                expected_output: None,
            }],
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::any_column("text", 0, "Text to tokenize (VARCHAR)"),
            ArgSpec::any_column("lang", 1, "Stemmer language, e.g. 'english' (VARCHAR)"),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(list_varchar_type()))
    }

    fn process(
        &self,
        params: &ProcessParams,
        batch: &arrow_array::RecordBatch,
    ) -> Result<arrow_array::RecordBatch> {
        let text_col = batch.column(0);
        let lang_col = batch.column(1);
        let rows = batch.num_rows();
        let mut builder = list_builder();
        for i in 0..rows {
            match (text_str(text_col, i)?, text_str(lang_col, i)?) {
                (Some(text), Some(lang)) => {
                    let toks = search::tokenize_lang(text, lang).map_err(ve)?;
                    append_list_row(&mut builder, &toks);
                }
                // NULL text or NULL language → NULL list.
                _ => append_list_null(&mut builder),
            }
        }
        let out = finish_list(builder);
        arrow_array::RecordBatch::try_new(params.output_schema.clone(), vec![out])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `stem(word, lang) -> VARCHAR` — Snowball-stem a single word.
pub struct Stem;

impl ScalarFunction for Stem {
    fn name(&self) -> &str {
        "stem"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Snowball-stem a single word for the given language (e.g. running → run)"
                .into(),
            return_type: Some(arrow_schema::DataType::Utf8),
            examples: vec![FunctionExample {
                sql: "SELECT tantivy.main.stem('running', 'english');".into(),
                description: "Snowball-stem a word to its root for a language ('running' → 'run')."
                    .into(),
                expected_output: None,
            }],
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::any_column("word", 0, "Word to stem (VARCHAR)"),
            ArgSpec::any_column("lang", 1, "Stemmer language, e.g. 'english' (VARCHAR)"),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(arrow_schema::DataType::Utf8))
    }

    fn process(
        &self,
        params: &ProcessParams,
        batch: &arrow_array::RecordBatch,
    ) -> Result<arrow_array::RecordBatch> {
        let word_col = batch.column(0);
        let lang_col = batch.column(1);
        let rows = batch.num_rows();
        let mut out = StringBuilder::new();
        for i in 0..rows {
            match (text_str(word_col, i)?, text_str(lang_col, i)?) {
                (Some(word), Some(lang)) => out.append_value(search::stem(word, lang).map_err(ve)?),
                // NULL word or NULL language → NULL.
                _ => out.append_null(),
            }
        }
        let arr: arrow_array::ArrayRef = Arc::new(out.finish());
        arrow_array::RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_io::test_support::{
        bound_type, run_scalar_on, run_scalar_text1, str_args, text_batch,
    };
    use arrow_array::cast::AsArray;
    use arrow_array::Array;
    use vgi::arguments::Arguments;

    #[test]
    fn tokenize_binds_list_and_splits() {
        assert_eq!(bound_type(&Tokenize), list_varchar_type());
        let out = run_scalar_text1(
            &Tokenize,
            &[Some("Running quickly, CATS!"), None],
            Arguments::default(),
        )
        .unwrap();
        let list = out.as_list::<i32>();
        let row = list.value(0);
        let strs = row.as_string::<i32>();
        assert_eq!(strs.len(), 3);
        assert_eq!(strs.value(0), "running");
        assert_eq!(strs.value(2), "cats");
        assert!(out.is_null(1), "NULL text → NULL list");
    }

    #[test]
    fn tokenize_lang_stems() {
        let batch = text_batch(&[&[Some("Running quickly")], &[Some("english")]]);
        let out = run_scalar_on(
            &TokenizeLang,
            batch,
            str_args(&[Some("Running quickly"), Some("english")]),
        )
        .unwrap();
        let row = out.as_list::<i32>().value(0);
        let strs = row.as_string::<i32>();
        assert_eq!(strs.value(0), "run");
    }

    #[test]
    fn stem_binds_varchar_and_stems() {
        assert_eq!(bound_type(&Stem), arrow_schema::DataType::Utf8);
        let batch = text_batch(&[&[Some("running")], &[Some("english")]]);
        let out =
            run_scalar_on(&Stem, batch, str_args(&[Some("running"), Some("english")])).unwrap();
        assert_eq!(out.as_string::<i32>().value(0), "run");
    }

    #[test]
    fn unknown_language_errors() {
        let batch = text_batch(&[&[Some("x")], &[Some("klingon")]]);
        let err = run_scalar_on(&Stem, batch, str_args(&[Some("x"), Some("klingon")]));
        assert!(err.is_err(), "unknown language must be a clear error");
    }
}
