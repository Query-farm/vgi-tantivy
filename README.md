# vgi-tantivy

A [VGI](https://query.farm) worker (Rust, a compiled binary) that brings
**full-text search** — BM25 relevance ranking — plus **tokenization** and
**Snowball stemming** to DuckDB / SQL over Apache Arrow, powered by the
[tantivy](https://github.com/quickwit-oss/tantivy) search engine. DuckDB launches
the worker and talks to it over Arrow IPC; the functions appear under the catalog
`tantivy`, schema `main`.

Every index is built in an **in-memory (RAM) directory per call** and dropped
when the call returns — the worker is stateless, nothing is persisted, and the
corpus is entirely caller-supplied (see [Ephemeral index](#ephemeral-index)).

```sql
LOAD vgi;
ATTACH 'tantivy' (TYPE vgi, LOCATION './target/release/tantivy-worker');
SET search_path = 'tantivy.main';

-- Rank a JSON document corpus by BM25 relevance (table function).
SELECT * FROM bm25_search('["the cat sat","dogs bark","stock market crash"]', 'cat');
--   doc_id | score
--        0 | 0.28768  (the cat doc ranks first)

-- Per-row text analysis (no index needed).
SELECT UNNEST(tokenize('Running quickly'));   -- 'running', 'quickly'
SELECT UNNEST(tokenize('Running quickly', 'english')); -- stemmed: 'run', 'quick'
SELECT stem('running', 'english');            -- → 'run'

-- Ad-hoc single-document relevance probe.
SELECT bm25_score('the cat sat on the mat', 'cat');  -- > 0
SELECT bm25_score('the stock market crashed', 'cat');-- 0.0 (no match)

-- Discovery.
SELECT * FROM supported_languages();          -- stemmer language ids
SELECT tantivy_version();                      -- 'tantivy v0.24.2, index_format v7'
```

## Functions

### Scalar (positional-only, per-row text analysis)

| Function | Returns | Description |
| --- | --- | --- |
| `tokenize(text)` | `VARCHAR[]` | Default tokenizer: unicode words, lowercased. |
| `tokenize(text, lang)` | `VARCHAR[]` | Tokenize + Snowball-stem for `lang`. |
| `stem(word, lang)` | `VARCHAR` | Snowball-stem a single word (`running` → `run`). |
| `bm25_score(doc_text, query)` | `DOUBLE` | Ad-hoc BM25 score of one document vs. a query (1-doc index; `0.0` if no match). English stemmer. |
| `tantivy_version()` | `VARCHAR` | tantivy engine + index-format version string. |

### Table (constant arguments, passed positionally)

| Function | Columns | Description |
| --- | --- | --- |
| `bm25_search(docs_json, query)` | `doc_id BIGINT, score DOUBLE` | BM25 ranking of a JSON corpus against a query, descending score. English stemmer. |
| `supported_languages()` | `lang VARCHAR` | Snowball stemmer language ids. |

> DuckDB table functions take **constant** arguments (no subqueries), so the
> corpus is handed to `bm25_search` as a single constant JSON payload — see the
> `docs_json` contract below. For per-row text analysis of a column, use the
> scalar functions (`tokenize`, `stem`, `bm25_score`).

## The `bm25_search` corpus: `docs_json`

`docs_json` is a constant JSON payload describing the corpus, in one of two forms:

* **array of strings** — `["the cat sat","dogs bark"]` — each document's `doc_id`
  is its 0-based array index; or
* **array of `{id,text}` objects** — `[{"id":42,"text":"a happy cat"}, …]` —
  carrying an explicit integer `id` returned as `doc_id`.

Because table functions take constants, assemble the payload from a real table
with `json_group_array`, materialize it into a variable, then pass the variable:

```sql
INSTALL json; LOAD json;

-- ids = 0-based array index:
SET VARIABLE docs = (SELECT json_group_array(body) FROM corpus);
SELECT * FROM bm25_search(getvariable('docs'), 'cat') ORDER BY score DESC;

-- explicit ids:
SET VARIABLE docs = (
  SELECT json_group_array(json_object('id', id, 'text', body)) FROM corpus);
SELECT * FROM bm25_search(getvariable('docs'), 'cat') ORDER BY score DESC;
```

## Ephemeral index

Both `bm25_search` and `bm25_score` build a brand-new tantivy index in a **RAM
directory** ([`Index::create_in_ram`]) for that one call, load the supplied
documents, run the query, and **drop the index when the call returns**. Nothing is
ever written to disk or shared across calls — there is no persistent or
incremental index. This keeps the worker stateless and the corpus fully
caller-controlled, at the cost of rebuilding the index per call (fine for the
modest corpora these functions target; input is bounded — see below).

`bm25_score` is deliberately a **1-document** index: it is an ad-hoc relevance
probe, so its scores are *not* comparable across documents/calls (BM25 statistics
depend on the whole corpus). To rank a real corpus, use `bm25_search`.

[`Index::create_in_ram`]: https://docs.rs/tantivy/latest/tantivy/struct.Index.html

## Supported languages

The Snowball stemmer languages (used by `tokenize(text, lang)`, `stem`, and the
search analyzer): `arabic`, `danish`, `dutch`, `english`, `finnish`, `french`,
`german`, `greek`, `hungarian`, `italian`, `norwegian`, `portuguese`, `romanian`,
`russian`, `spanish`, `swedish`, `tamil`, `turkish` (common ISO aliases like `en`,
`de`, `fr` are accepted). An unknown language is a clear error. The corpus-ranking
functions use the English stemmer.

## Behavior & robustness

* Text is **data**, never executed.
* Input is **bounded**: 16 MiB per text value, 1,000,000 documents per corpus.
* `NULL` / empty input → `NULL` (scalars) or no rows (table functions); an empty
  corpus or blank query yields no hits.
* BM25 scores are **deterministic** for a fixed corpus; ranking order is stable
  (ties broken by ascending `doc_id`).
* An **unknown language**, **malformed `docs_json`**, or an **unparseable query**
  is a clear DuckDB error (all caller mistakes). The worker never panics.

## Building & testing

```sh
cargo build --release                                    # build the worker
cargo test --workspace                                   # unit + integration tests
cargo clippy --all-targets --all-features -- -D warnings # lint
cargo fmt --all -- --check                               # format check
make test-sql                                            # DuckDB SQL end-to-end
```

`make test-sql` builds the release worker, points `VGI_TANTIVY_WORKER` at it, and
runs the [`haybarn-unittest`](https://pypi.org/project/haybarn-unittest/)
sqllogictest suite under `test/sql/`. Install the runner once with
`uv tool install haybarn-unittest`.

## Licensing of dependencies

The worker is MIT (see [LICENSE](LICENSE)). [`tantivy`](https://crates.io/crates/tantivy)
— the full-text search engine, including its tokenizers and the Snowball stemmer
stack — is MIT-licensed, compatible with this project's MIT license.

## License

MIT — see [LICENSE](LICENSE).
