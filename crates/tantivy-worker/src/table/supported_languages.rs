//! `supported_languages() -> (lang VARCHAR)` — the Snowball/stemmer language ids
//! the worker supports (for `tokenize`, `stem`, and the search analyzer), for
//! discovery.

use std::sync::Arc;

use arrow_array::builder::StringBuilder;
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use vgi::table_function::{TableFunction, TableProducer};
use vgi::{ArgSpec, BindParams, BindResponse, FunctionExample, FunctionMetadata, ProcessParams};
use vgi_rpc::{OutputCollector, Result, RpcError};

use crate::search;

pub struct SupportedLanguages;

fn output_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![Field::new("lang", DataType::Utf8, false)]))
}

impl TableFunction for SupportedLanguages {
    fn name(&self) -> &str {
        "supported_languages"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "List the Snowball stemmer language ids this worker supports".into(),
            examples: vec![FunctionExample {
                sql: "SELECT * FROM tantivy.main.supported_languages();".into(),
                description: "List the Snowball stemmer language ids usable with `tokenize`, \
                              `stem`, and the search analyzer."
                    .into(),
                expected_output: None,
            }],
            tags: {
                let mut tags = crate::meta::object_tags(
                    "Supported Stemmer Languages",
                    "List the Snowball stemmer language ids this worker supports. These are the \
                     valid `lang` values for tokenize(text, lang), stem(word, lang), and the \
                     internal search analyzer. Use it to discover which languages are available.",
                    "List the supported Snowball stemmer language ids, usable with `tokenize`, \
                     `stem`, and the search analyzer. Column: `lang`.",
                    "supported languages, stemmer languages, snowball languages, available \
                     languages, language list, discovery, tokenize languages, stem languages",
                    "table/supported_languages.rs",
                );
                tags.push((
                    "vgi.result_columns_md".into(),
                    "| column | type | description |\n\
                     |---|---|---|\n\
                     | `lang` | VARCHAR | A supported Snowball stemmer language id, e.g. `english`, \
                     `french`, `german`. |"
                        .into(),
                ));
                tags
            },
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        Vec::new()
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse {
            output_schema: output_schema(),
            opaque_data: Vec::new(),
        })
    }

    fn producer(&self, params: &ProcessParams) -> Result<Box<dyn TableProducer>> {
        Ok(Box::new(LangsProducer {
            schema: params.output_schema.clone(),
            done: false,
        }))
    }
}

struct LangsProducer {
    schema: SchemaRef,
    done: bool,
}

impl TableProducer for LangsProducer {
    fn next_batch(&mut self, _out: &mut OutputCollector) -> Result<Option<RecordBatch>> {
        if self.done {
            return Ok(None);
        }
        self.done = true;
        let mut b = StringBuilder::new();
        for l in search::SUPPORTED_LANGUAGES {
            b.append_value(l);
        }
        let col: ArrayRef = Arc::new(b.finish());
        Ok(Some(
            RecordBatch::try_new(self.schema.clone(), vec![col])
                .map_err(|e| RpcError::runtime_error(e.to_string()))?,
        ))
    }
}
