# CLAUDE.md — vgi-tantivy

Contributor/agent notes. User-facing docs live in `README.md`; this is the
"how it's built and where the sharp edges are" companion.

## What this is

A [VGI](https://query.farm) worker (Rust, compiled binary) exposing **full-text
search** (BM25 ranking) plus **tokenization / Snowball stemming** to DuckDB/SQL
over Arrow IPC. Built on the `vgi` crate (crates.io), modeled on `vgi-code` /
`vgi-image`. Catalog name `tantivy` (single `main` schema). Search powered by
[`tantivy`](https://crates.io/crates/tantivy).

## Layout

```
Cargo.toml                          workspace; pins vgi = "0.9.0", tantivy = "=0.24.2"
crates/tantivy-worker/
  src/main.rs                       Worker::new(); registers scalars + tables
  src/search.rs                     PURE logic (no Arrow): language registry, tokenize/stem,
                                    docs_json parsing, ephemeral RAM-index BM25 search + unit tests
  src/arrow_io.rs                   VARCHAR cell reads + LIST(VARCHAR) builder + in-process scalar harness
  src/scalar/{analyze,score,version,mod}.rs    thin Arrow scalar adapters
  src/table/{bm25_search,supported_languages,mod}.rs   thin Arrow table-producer adapters
  tests/search.rs                   integration tests over `search` (corpus ranking, tokenize, stem)
test/sql/*.test                     haybarn-unittest sqllogictest — authoritative E2E
Makefile                            test / test-unit / test-sql / lint / fmt / build / clean
```

Pattern: keep computation in `search.rs` (pure, unit-tested), keep Arrow
marshalling in `arrow_io.rs` + `scalar/*.rs` + `table/*.rs` (thin, harness-tested).

## Library: tantivy

`tantivy = "=0.24.2"` is the full-text search engine. We use:
- its tokenizer stack (`SimpleTokenizer` + `LowerCaser` + `RemoveLongFilter`) and
  the Snowball `Stemmer` for `tokenize`/`stem` and the search analyzer;
- an **in-RAM** index (`Index::create_in_ram`) + `QueryParser` + `TopDocs`
  collector for BM25 ranking;
- `tantivy::version_string()` for `tantivy_version()`.

## Sharp edges (learned the hard way)

1. **MSRV pinning (the fiddly one).** Workspace `rust-version = 1.86`. `tantivy`
   `0.25`/`0.26` bump the toolchain past 1.86, so we pin `tantivy = "=0.24.2"`
   (builds cleanly on 1.86). Its transitive chain still tries to resolve newer
   crates that require Rust ≥ 1.87/1.88, so `Cargo.lock` is pinned back with
   `cargo update -p <c> --precise <v>`:
   - `time` → `0.3.41` (pulls `time-core` 0.1.4, `time-macros` 0.2.22)
   - `darling` → `0.20.11`
   - `wasip2` → `1.0.0+wasi-0.2.4`
   These keep the whole graph ≤ 1.86. If you bump `tantivy`, re-check these.

2. **`version_string()` returns `&str` in tantivy 0.24** — not `String`. The pure
   `tantivy_version()` does `.to_string()`.

3. **Ephemeral RAM index per call.** Every `bm25_search` / `bm25_score` call
   builds a fresh `Index::create_in_ram`, registers the per-language analyzer
   under one tokenizer name, indexes the docs, commits, searches, and drops it.
   Nothing persists. `bm25_score` is intentionally a 1-doc index (ad-hoc probe;
   scores not corpus-comparable).

4. **`docs_json` is the corpus.** Table functions take constants, not subqueries,
   so the corpus arrives as a constant JSON payload: an array of strings (id =
   index) or `[{"id","text"}]`. Parsed in `search::parse_docs`. In SQL, assemble
   with `json_group_array`, materialize into a `SET VARIABLE`, pass `getvariable`
   (DuckDB rejects a subquery directly in a table-function arg).

5. **Two `tokenize` overloads share a name.** `Tokenize` (1 arg) and
   `TokenizeLang` (2 args) both `name() == "tokenize"`; the SDK stores scalars as
   `HashMap<String, Vec<…>>` and dispatches by arity, so this is fine.

6. **`haybarn-unittest` skips `require vgi`** — `.test` files use explicit
   `statement ok` + `LOAD vgi;`. Functions live under the `tantivy` catalog, so
   each file does `SET search_path = 'tantivy.main'`, then `USE memory` before
   `DETACH tantivy`. The search E2E also `INSTALL json; LOAD json;` for the
   `json_group_array` assembly demo (the runner's DuckDB doesn't autoload it).
   `rowsort` query blocks must list expected rows in **sorted** order.

7. **LIST(VARCHAR) return type must match between bind and process.** The Arrow
   `DataType::List` published in `on_bind` must exactly equal the array built in
   `process` (child field `item`, nullable). Both go through
   `arrow_io::list_varchar_type()` / `list_builder()` so they cannot drift.

8. **Scalars are positional-only; table functions take constants.** `tokenize`/
   `stem`/`bm25_score` read per-row VARCHAR columns; `bm25_search` reads bind-time
   `const_str(i)` and validates `docs_json` + query at `on_bind` for a clear early
   error. `BM25` scores are deterministic given the corpus — the `.test` suite
   asserts ranking ORDER (top doc_ids) and uses `>`/`= 0.0` comparisons rather
   than exact floats.

9. **Robustness.** Input bounded (`MAX_TEXT_BYTES = 16 MiB`, `MAX_DOCS = 1e6`).
   NULL/empty → NULL/no rows; empty corpus/blank query → no hits. Hard errors:
   unknown language, malformed `docs_json`, unparseable query — all caller
   mistakes, surfaced clearly. Never panics on input.

## Testing

```sh
cargo test --workspace                                   # pure unit + arrow-boundary harness + integration
cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
make test-sql                                            # builds release, sets VGI_TANTIVY_WORKER, haybarn over test/sql/*
make test                                                # cargo test + sql
```

CI (`.github/workflows/ci.yml`) runs fmt/clippy/build/test plus a gated
`e2e-sql` job (installs `uv` + `haybarn-unittest`, runs `make test-sql`).

## Function surface

Scalars: `tokenize` (1- and 2-arg), `stem`, `bm25_score`, `tantivy_version`.
Tables: `bm25_search`, `supported_languages`. Garbage / empty / oversized input →
graceful empty / NULL / no rows; an unknown language, malformed `docs_json`, or
unparseable query is a clear error.
