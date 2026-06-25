//! `tantivy_version() -> VARCHAR` — the tantivy engine version string (and index
//! format), e.g. `"tantivy v0.24.2, index_format v7"`.

use std::sync::Arc;

use arrow_array::{ArrayRef, RecordBatch, StringArray};
use arrow_schema::DataType;
use vgi::{
    ArgSpec, BindParams, BindResponse, FunctionExample, FunctionMetadata, ProcessParams,
    ScalarFunction,
};
use vgi_rpc::{Result, RpcError};

pub struct TantivyVersion;

impl ScalarFunction for TantivyVersion {
    fn name(&self) -> &str {
        "tantivy_version"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Returns the tantivy engine version and index-format string".into(),
            return_type: Some(DataType::Utf8),
            examples: vec![FunctionExample {
                sql: "SELECT tantivy.main.tantivy_version();".into(),
                description: "Return the tantivy engine version and index-format string.".into(),
                expected_output: None,
            }],
            tags: crate::meta::object_tags(
                "Tantivy Engine Version",
                "Return the version string of the underlying tantivy full-text search engine and \
                 its on-disk index format, e.g. 'tantivy v0.24.2, index_format v7'. Use it to \
                 confirm which search-engine version backs this worker.",
                "Return the tantivy engine version and index-format string, e.g. \
                 `tantivy v0.24.2, index_format v7`.",
                &[
                    "tantivy version",
                    "engine version",
                    "index format",
                    "build info",
                    "version string",
                    "about",
                    "diagnostics",
                ],
            ),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        Vec::new()
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Utf8))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let rows = batch.num_rows();
        let v = crate::search::tantivy_version();
        let out: ArrayRef = Arc::new(StringArray::from(vec![v; rows]));
        RecordBatch::try_new(params.output_schema.clone(), vec![out])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_io::test_support::run_scalar_text1;
    use arrow_array::cast::AsArray;
    use vgi::arguments::Arguments;

    #[test]
    fn returns_tantivy_version() {
        let out =
            run_scalar_text1(&TantivyVersion, &[Some("ignored")], Arguments::default()).unwrap();
        assert!(out.as_string::<i32>().value(0).contains("tantivy"));
    }
}
