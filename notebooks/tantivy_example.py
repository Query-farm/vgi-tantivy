"""Result checks and SVG presentation for the authored Tantivy search corpus."""

import matplotlib.pyplot as plt
import numpy as np
from IPython.display import display
from matplotlib.colors import ListedColormap
from vgi_notebooks import finish, prose


def validate_parameter(require_all):
    if type(require_all) is not bool:
        raise ValueError("REQUIRE_ALL_TERMS must be True or False.")


def analyzer_choice(stems, availability):
    display(availability, stems)
    assert len(stems) == 4 and set(stems.lang) == {"english"}
    values = stems.set_index("word").stem.to_dict()
    assert values["battery"] == values["batteries"]
    prose(
        f"The catalog advertises **{int(availability.available_languages.iloc[0])} stemmer languages** and confirms "
        "that **English is available**. We use it consistently with this worker's search analyzer. "
        f"`battery` and `batteries` both become **`{values['battery']}`**. However, `storage` becomes "
        f"**`{values['storage']}`** while `storing` becomes **`{values['storing']}`**. "
        "Stemming is a language-specific normalization rule, not synonym matching or automatic language detection."
    )


def coverage_chart(coverage):
    matrix = coverage.pivot(index="doc_id", columns="term", values="occurrences")
    fig, ax = plt.subplots(figsize=(11, 7.6))
    ax.imshow(
        matrix.values,
        cmap=ListedColormap(["#e4e8e5", "#b7ddcc", "#6daf92", "#267358"]),
        vmin=0,
        vmax=3,
        aspect="auto",
    )
    ax.set_xticks(range(len(matrix.columns)), matrix.columns)
    ax.set_yticks(range(len(matrix)), [f"Document {i}" for i in matrix.index])
    ax.set_xticks(np.arange(-0.5, len(matrix.columns), 1), minor=True)
    ax.set_yticks(np.arange(-0.5, len(matrix), 1), minor=True)
    ax.grid(False)
    ax.grid(which="minor", color="white", linewidth=3)
    ax.tick_params(which="minor", bottom=False, left=False)
    for y, row in enumerate(matrix.values):
        for x, value in enumerate(row):
            ax.text(
                x,
                y,
                str(int(value)),
                ha="center",
                va="center",
                fontsize=12,
                color="white" if value == 3 else "#253d31",
            )
    finish(
        fig,
        "Stemming defines the terms available to the query",
        "English query stems · cells count occurrences in each indexed document",
        "Source: ten authored input records · eight indexed texts · native VGI Tantivy v0.4.0 · English analyzer",
        name="01-term-coverage",
    )
    both = int((matrix > 0).all(axis=1).sum())
    either = int((matrix > 0).any(axis=1).sum())
    prose(
        f"**{both} indexed documents contain both query stems**, while **{either} contain at least one**. "
        "Repeated battery terms add occurrences but do not satisfy a missing storage term. The grid stays fixed when only the query operator changes."
    )
    return matrix, both, either


def ranking_chart(ranked, matrix, query):
    fig, ax = plt.subplots(figsize=(11, 6.7))
    labels = [f"{int(r.rank)}. Document {r.doc_id}" for r in ranked.itertuples()]
    colors = [
        "#267358" if len(r.matching_terms) == len(matrix.columns) else "#c57b40"
        for r in ranked.itertuples()
    ]
    bars = ax.barh(labels, ranked.score, color=colors, height=0.6)
    ax.bar_label(bars, labels=[f"{s:.3f}" for s in ranked.score], padding=7, fontsize=11)
    ax.invert_yaxis()
    ax.set_xlim(0, float(ranked.score.max()) * 1.18)
    ax.set_xlabel("BM25 score · corpus-specific, unitless")
    ax.grid(axis="x")
    finish(
        fig,
        "The query operator determines who reaches the ranking",
        f"{query} · green = both stems; amber = one stem",
        "Source: same authored eight-text corpus · native VGI Tantivy v0.4.0 · score ties broken by document ID",
        name="02-ranked-hits",
    )
    display(ranked)
    prose(
        f"The query returns **{len(ranked)} of {len(matrix)} indexed documents**. "
        f"Document **{int(ranked.iloc[0].doc_id)}** ranks first in this run. "
        "Consult the joined text and matching terms before interpreting the rank. A lexical match does not establish that the document answers a question."
    )


def score_context(probe):
    display(probe)
    row = probe.iloc[0]
    prose(
        f"Document **{int(row.doc_id)}** scores **{row.shared_corpus_score:.3f}** in the shared corpus and "
        f"**{row.one_document_score:.3f}** in its isolated one-document index. "
        "These are separate scoring contexts. The scalar probe does not calibrate or replace the shared-corpus ranking."
    )


def check_results(checks, ranked, both, either, require_all, actual_hits):
    display(checks)
    assert checks.passed.fillna(False).all(), checks.loc[~checks.passed.fillna(False)]
    assert len(ranked) == (both if require_all else either)
    assert len(ranked) == actual_hits
    prose(
        f"**{len(checks)} SQL checks and two hit-count reconciliations passed.** "
        "The demonstration checks retrieval and joins, not semantic answer quality."
    )
