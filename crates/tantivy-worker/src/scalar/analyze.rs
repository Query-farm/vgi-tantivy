//! Per-row text analysis scalars (no index needed):
//! - `tokenize(text) -> VARCHAR[]` — default tokenizer (unicode words, lowercased).
//! - `tokenize(text, lang) -> VARCHAR[]` — language stemming variant.
//! - `stem(word, lang) -> VARCHAR` — Snowball-stem a single word.
//!
//! `lang` is a per-row VARCHAR column. NULL text → NULL output; an unknown
//! language is a clear error (the caller named it).

use std::sync::Arc;

use arrow_array::builder::StringBuilder;
use vgi::{ArgSpec, BindParams, BindResponse, FunctionMetadata, ProcessParams, ScalarFunction};
use vgi_rpc::{Result, RpcError};

use crate::arrow_io::{
    append_list_null, append_list_row, finish_list, list_builder, list_varchar_type, text_str,
};
use crate::search;

fn ve(e: impl std::fmt::Display) -> RpcError {
    RpcError::value_error(e.to_string())
}

// `tokenize` has two overloads that share one name (dispatched by arity), so the
// linter sees a single `tokenize` object. Both overloads therefore publish the
// SAME docs — describing BOTH forms coherently — plus BOTH worked examples, so
// whichever the merged view surfaces is consistent (VGI180).
const TOKENIZE_TITLE: &str = "Tokenize Text";
const TOKENIZE_DOC_LLM: &str =
    "Split text into normalized word tokens, returned as a `VARCHAR` array. \
     Two forms: `tokenize(text)` applies the default analyzer only — unicode word segmentation, \
     lowercasing, and dropping of overly long tokens, with NO stemming (e.g. 'Running quickly, \
     CATS!' → ['running','quickly','cats']); `tokenize(text, lang)` does the same and additionally \
     Snowball-stems each token for the given language, collapsing word variants to a shared root \
     (e.g. 'Running quickly' with 'english' → ['run','quickli']). NULL text (or NULL language in \
     the 2-arg form) → NULL; an unknown language is a clear error.";
const TOKENIZE_DOC_MD: &str =
    "Tokenize text into a `VARCHAR` array of normalized terms. The 1-arg \
     form `tokenize(text)` only tokenizes and lowercases (no stemmer), e.g. \
     `tokenize('Running quickly, CATS!')` → `['running','quickly','cats']`. The 2-arg form \
     `tokenize(text, lang)` additionally Snowball-stems each token, e.g. \
     `tokenize('Running quickly', 'english')` → `['run','quickli']`.";
// Canonical, self-contained runnable examples for the `tokenize` object, so an
// analyst agent converges on the right call for both arities (VGI920/VGI509).
const TOKENIZE_EXECUTABLE_EXAMPLES: &str = r#"[
  {
    "description": "Tokenize text into lowercased word tokens (1-arg form, no stemming).",
    "sql": "SELECT tantivy.main.tokenize('Running quickly, CATS!') AS tokens"
  },
  {
    "description": "Tokenize and Snowball-stem text for a language (2-arg form).",
    "sql": "SELECT tantivy.main.tokenize('Running quickly', 'english') AS tokens"
  }
]"#;
// Illustrative `vgi.example_queries` (VGI515/VGI306) for the `tokenize` object.
// Both arity overloads publish the SAME aggregated pair list (by function name)
// so the merged catalog view is consistent, and the native `FunctionExample`
// carrier drops descriptions where this tag preserves them.
const TOKENIZE_EXAMPLE_QUERIES: &[(&str, &str)] = &[
    (
        "Tokenize text into lowercased word tokens (1-arg form, no stemming).",
        "SELECT tantivy.main.tokenize('Running quickly, CATS!') AS tokens",
    ),
    (
        "Tokenize and Snowball-stem text for a language (2-arg form).",
        "SELECT tantivy.main.tokenize('Running quickly', 'english') AS tokens",
    ),
];
const TOKENIZE_KEYWORDS: &[&str] = &[
    "tokenize",
    "tokenization",
    "tokens",
    "split words",
    "word segmentation",
    "lowercase",
    "stemming tokenizer",
    "snowball",
    "analyzer",
    "multilingual",
    "text analysis",
    "terms",
];

/// `tokenize(text) -> VARCHAR[]` — default (non-stemming) tokenizer.
pub struct Tokenize;

impl ScalarFunction for Tokenize {
    fn name(&self) -> &str {
        "tokenize"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description:
                "Tokenize text into VARCHAR[]; 1-arg form only splits+lowercases (no stemming)"
                    .into(),
            return_type: Some(list_varchar_type()),
            examples: crate::meta::examples_from(TOKENIZE_EXAMPLE_QUERIES),
            tags: {
                let mut tags = crate::meta::object_tags(
                    TOKENIZE_TITLE,
                    TOKENIZE_DOC_LLM,
                    TOKENIZE_DOC_MD,
                    TOKENIZE_KEYWORDS,
                    "Text Analysis",
                );
                tags.push((
                    "vgi.executable_examples".into(),
                    TOKENIZE_EXECUTABLE_EXAMPLES.into(),
                ));
                tags.push(crate::meta::example_queries_tag(TOKENIZE_EXAMPLE_QUERIES));
                tags
            },
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![ArgSpec::column(
            "text",
            0,
            "varchar",
            "The text to split into tokens",
        )]
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
                "Tokenize text into VARCHAR[]; 2-arg form additionally Snowball-stems each token"
                    .into(),
            return_type: Some(list_varchar_type()),
            examples: crate::meta::examples_from(TOKENIZE_EXAMPLE_QUERIES),
            tags: {
                let mut tags = crate::meta::object_tags(
                    TOKENIZE_TITLE,
                    TOKENIZE_DOC_LLM,
                    TOKENIZE_DOC_MD,
                    TOKENIZE_KEYWORDS,
                    "Text Analysis",
                );
                tags.push((
                    "vgi.executable_examples".into(),
                    TOKENIZE_EXECUTABLE_EXAMPLES.into(),
                ));
                tags.push(crate::meta::example_queries_tag(TOKENIZE_EXAMPLE_QUERIES));
                tags
            },
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::column("text", 0, "varchar", "The text to split into tokens"),
            ArgSpec::column(
                "lang",
                1,
                "varchar",
                "Snowball stemmer language to apply to each token, e.g. 'english'; \
                 call supported_languages() for the valid ids",
            ),
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

/// Shared `(description, sql)` example for `stem`, used byte-identically for the
/// native `FunctionExample` carrier and the `vgi.example_queries` tag (VGI515).
const STEM_EXAMPLE_QUERIES: &[(&str, &str)] = &[(
    "Snowball-stem a single English word to its root.",
    "SELECT tantivy.main.stem('running', 'english') AS root",
)];

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
            examples: crate::meta::examples_from(STEM_EXAMPLE_QUERIES),
            tags: {
                let mut tags = crate::meta::object_tags(
                    "Stem Single Word",
                    "Reduce a single word to its Snowball stem (root) for the given language, e.g. \
                     'running' → 'run' in English. Useful for normalizing terms before matching or \
                     grouping. NULL word or language → NULL; an unknown language is a clear error.",
                    "Snowball-stem one word to its root for a language, e.g. \
                     `stem('running', 'english')` → `run`.",
                    &[
                        "stem",
                        "stemming",
                        "snowball",
                        "root word",
                        "lemmatize",
                        "normalize term",
                        "word root",
                        "morphology",
                        "language",
                    ],
                    "Text Analysis",
                );
                tags.push((
                    "vgi.executable_examples".into(),
                    r#"[
  {
    "description": "Snowball-stem a single English word to its root.",
    "sql": "SELECT tantivy.main.stem('running', 'english') AS root"
  }
]"#
                    .into(),
                ));
                tags.push(crate::meta::example_queries_tag(STEM_EXAMPLE_QUERIES));
                tags
            },
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::column(
                "word",
                0,
                "varchar",
                "The single word to reduce to its stem",
            ),
            ArgSpec::column(
                "lang",
                1,
                "varchar",
                "Snowball stemmer language to stem the word with, e.g. 'english'; \
                 call supported_languages() for the valid ids",
            ),
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
