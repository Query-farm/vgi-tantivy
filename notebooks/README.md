# Tantivy search examples

| Notebook | Question | Run |
|---|---|---|
| [01 · Retrieval scope comes before ranking](01_retrieval_scope_comes_before_ranking.ipynb) | Which documents can an English AND/OR query retrieve before we interpret its BM25 scores? | [Open in Colab](https://colab.research.google.com/github/Query-farm/vgi-tantivy/blob/main/notebooks/01_retrieval_scope_comes_before_ranking.ipynb) |

The example checks the supported language vocabulary, compares English word stems,
inspects token occurrences, searches one authored corpus, and joins every hit to its
source ID. It also shows why a one-document scalar score is a separate context.
The visible `github-auto://Query-farm/vgi-tantivy@v0.4.0` ATTACH downloads a verified
native release; no local Rust build or account is required.

In Colab choose **Runtime → Run all**. The base shared package retrieves immutable,
hash-verified helper and corpus files without cloning the worker checkout. Compatible
preinstalled kernel and plotting libraries remain installed. Local publication uses
the `[runner]` extra and this directory's lockfile:

```sh
uv sync --locked
uv run --locked vgi-notebooks
uv run --locked vgi-notebooks 01_retrieval_scope_comes_before_ranking.ipynb --parameters '{"REQUIRE_ALL_TERMS":false}' --output-dir artifacts/or-query
```

Select `notebooks/.venv/bin/python` in an interactive local Jupyter environment. The
runner creates its own temporary kernel, rejects overrides without a separate output
and preserves successful defaults on failure. If an older setup replaced Colab's
kernel libraries, save your work, choose **Runtime → Disconnect and delete runtime**,
reconnect and open this updated notebook.

The [shared standard](https://github.com/Query-farm/vgi-example-notebooks/blob/v0.3.3/docs/standard.md),
runner, canonical branding, SQL secret isolation and SVG presentation live in
[vgi-example-notebooks](https://github.com/Query-farm/vgi-example-notebooks).
`tantivy_example.py` contains result checks, charts and interpretations. Analytical
SQL stays visible in the notebook. Commit and retain `charts/` SVG assets for GitHub;
Jupyter/Colab reruns use native SVG. PNG files under `artifacts/review/` are optional
review exports rather than the publication format.

`fixtures/corpus.json` contains ten authored records: eight nonempty document strings,
one empty string and one missing value. They are learning examples, not external
research or product specifications. The notebook explicitly excludes empty and missing
text from its search corpus while retaining both in the input inventory. Stable IDs
support complete result joins. `fixtures/manifest.json` records the hash; regenerate
identical fixtures locally with `uv run --locked python fixtures/build.py`.

English stemming normalizes `battery` and `batteries` together but does not equate
`storage` and `storing`. The available-language table supplies valid configuration
choices, not language detection. The two-term grid explains this example's plain
query eligibility; it is not a general replacement for Tantivy's query parser.

Change only `REQUIRE_ALL_TERMS` from `True` to `False` to compare `battery storage`
with `battery OR storage`. The corpus, chosen analyzer, word stems and occurrence
grid remain fixed. The original conjunction returns three of eight indexed documents;
OR returns seven, adding four single-term documents. Compare the returned IDs and
text before interpreting scores. Reset `True` to reproduce the saved default.

Each worker call builds and discards an in-memory index. SQL materializes one search
before plots reuse it. SQL assembles the constant `{id,text}` JSON; a short quoting
bridge passes that exact payload without relying on cursor-local session variables.
`bm25_score` builds a separate one-document index, so its score is not interchangeable
with the shared-corpus result. Scores are not probabilities or correctness judgments;
a NULL score in the left join is absence of a returned hit, not a measured zero.
The worker caps returned hits at 10,000; this example indexes only eight documents.

`coverage.json` binds successful execution, visible SQL and editorial review to full
release/current catalog snapshots. All five public named surfaces are meaningfully
used. The notebook actually executes v0.4.0; current-interface coverage is separately
reviewed against matching catalog signatures. Upgrade the release pin only after
repeating both parameter modes and checking catalog drift.

Defaults, a detached OR experiment, all four chart views, three runner guards and a
notebook-only bootstrap are checked. An actual kernel with already-imported
Colab-compatible libraries is tested separately from a manually operated hosted
Colab session; no hosted session is claimed. Worker source and root locks are unchanged.
