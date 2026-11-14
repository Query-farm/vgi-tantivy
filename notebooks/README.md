# VGI Tantivy notebooks

[01 · Retrieval scope comes before ranking](01_retrieval_scope_comes_before_ranking.ipynb)
uses visible Haybarn SQL to inspect English tokens, build a small document corpus,
retrieve and rank hits, and join them to the complete input inventory. Two charts
show query-stem occurrences and corpus BM25 scores. The saved notebook is executed.

## Run

Install `uv`, then from this directory:

```sh
uv sync --locked
uv run --locked python execute.py
```

The whole `notebooks/` directory works when copied elsewhere; no worker checkout or
Rust build is required. The runner uses this directory as its working directory
and a temporary kernel, replacing saved outputs only after success. Internet access
is required initially for dependencies, the community VGI extension and native
release. Your platform needs a matching release asset. Tested on macOS Apple silicon
with Haybarn 2.0.0rc11.

For VS Code, select this directory's `.venv/bin/python` as the kernel, or register it:

```sh
uv run --locked python -m ipykernel install --user --name vgi-tantivy-notebooks --display-name 'Python (vgi-tantivy notebooks)'
```

Use this directory as the working directory when running interactively. Parameters,
connection SQL and analytical SQL stay visible; preparation, plots and cleanup are
folded. Restart and run all after changing `REQUIRE_ALL_TERMS`.

## Experiment

```sh
uv run --locked python execute.py --parameters '{"REQUIRE_ALL_TERMS": false}' --output-dir validation/or-query
```

Overrides require a separate destination, which receives branding, fixtures and
environment companions. The conjunctive `battery storage` query returns three
of eight indexed documents. `battery OR storage` returns seven. The corpus and
term-occurrence grid are unchanged; four single-term documents become eligible.
Reset `True` to restore the saved conjunctive query.

## Worker and corpus provenance

The visible literal location is
`github-auto://Query-farm/vgi-tantivy@v0.4.0`. It downloads a native release, checks its
hash and caches it. To upgrade, edit the release pin and verify both parameter
settings. Notebook dependencies have a separate locked environment.

`fixtures/corpus.json` contains ten authored records: eight nonempty short document
strings, one empty string and one missing text value. They are learning examples,
not external research or product specifications. Stable IDs are retained when hits
are joined to source rows. `fixtures/manifest.json` records the file hash; regenerate
identical data with:

```sh
uv run --locked python fixtures/build.py
```

SQL explicitly excludes empty and missing text from this example's corpus, while
retaining both in the input inventory. SQL constructs the ordered `{id,text}` JSON
array. Folded preparation quotes the exact materialized JSON and query for the
visible constant-argument table call, avoiding JupySQL cursor-local session variables.
The call is materialized once before tables or charts reuse the search result.

## Interpretation and validation

Search uses an English analyzer; two-argument `tokenize` exposes those stemmed terms.
The coverage grid explains eligibility for the two simple plain query terms in
this example. It is not a replacement for the full query parser. The engine creates
and discards an in-memory index for each search. Its returned-hit cap is 10,000;
this corpus has eight indexed documents. Source bounds are 16 MiB per text value
and one million input documents; this is a small-corpus demonstration.

A missing hit produces NULL in the left-joined score column, not a measured zero.
Scores are corpus-specific and are not probabilities, correctness judgments or
universal relevance thresholds. `bm25_score` creates a one-document index for each
scalar call; its statistics differ from the shared-corpus search. The notebook
shows that distinction for one hit and never ranks the corpus by independent
scalar probes. Exact floating-point scores are not asserted.

Default and detached OR-query runs pass twelve SQL checks and two hit-count
reconciliations. Both charts are inspected in both modes. The fixture regenerates
identically and matches its manifest. Runner guards cover overrides without an
output directory, unknown parameters and invalid-parameter failure preserving
saved outputs.

Repository validation passes locked release build, 44 Rust tests, Clippy, formatting,
warning-clean Rust documentation, and 90/94/90 SQL assertions across subprocess,
HTTP and Unix transports (two cases each). Metadata scores 100/100, L2, zero
findings, with structural and behavioral checks verified. Root source and dependency
files remain unchanged.

Official Query.Farm Strata Sun and VGI assets are bundled unchanged. The wordmark
links to Query.Farm at both the top and final bottom of the notebook.
