//! Integration tests over the pure search/analysis surface. These exercise the
//! same `search` functions the Arrow adapters call, but as a standalone consumer
//! of the crate's library-style API.
//!
//! The worker is a binary crate, so to test `search` as an integration test we
//! include the source module directly. It has no dependencies on the Arrow layer.

#[path = "../src/search.rs"]
mod search;

use search::{
    bm25_score, bm25_search, parse_docs, stem, tantivy_version, tokenize_default, tokenize_lang,
    Document, SearchError, SUPPORTED_LANGUAGES,
};

/// A small corpus: two cat docs, one dog doc, one finance doc.
fn corpus() -> Vec<Document> {
    parse_docs(
        r#"[
          "the cat sat quietly on the warm mat",
          "a curious cat chased a red laser across the floor",
          "the loyal dog barked loudly at the mailman",
          "the stock market crashed and global finance suffered"
        ]"#,
    )
    .unwrap()
}

#[test]
fn cat_query_ranks_cat_docs_above_finance() {
    let hits = bm25_search(&corpus(), "cat", "english", 10).unwrap();
    // Only the two cat docs (ids 0 and 1) match a single-term 'cat' query.
    assert_eq!(hits.len(), 2);
    let ids: Vec<i64> = hits.iter().map(|h| h.doc_id).collect();
    assert!(ids.contains(&0) && ids.contains(&1), "cat docs present");
    assert!(!ids.contains(&3), "finance doc must not rank for 'cat'");
    // Scores are strictly descending.
    for w in hits.windows(2) {
        assert!(w[0].score >= w[1].score, "scores descending");
    }
    assert!(hits[0].score > 0.0);
}

#[test]
fn finance_query_ranks_finance_doc_first() {
    let hits = bm25_search(&corpus(), "finance market", "english", 10).unwrap();
    assert_eq!(hits[0].doc_id, 3, "finance doc ranks top for finance query");
}

#[test]
fn explicit_ids_round_trip() {
    let docs =
        parse_docs(r#"[{"id":100,"text":"a sleepy cat"},{"id":200,"text":"a barking dog"}]"#)
            .unwrap();
    let hits = bm25_search(&docs, "cat", "english", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].doc_id, 100);
}

#[test]
fn empty_corpus_and_empty_query_yield_no_rows() {
    assert!(bm25_search(&[], "cat", "english", 10).unwrap().is_empty());
    assert!(bm25_search(&corpus(), "", "english", 10)
        .unwrap()
        .is_empty());
}

#[test]
fn bm25_score_relevant_exceeds_irrelevant() {
    let relevant = bm25_score("the cat sat on the mat", "cat", "english").unwrap();
    let irrelevant = bm25_score("the stock market crashed", "cat", "english").unwrap();
    assert!(relevant > irrelevant);
    assert_eq!(irrelevant, 0.0);
}

#[test]
fn tokenize_default_and_lang() {
    assert_eq!(
        tokenize_default("Running quickly").unwrap(),
        vec!["running", "quickly"]
    );
    // English stemming collapses 'running' → 'run'.
    assert_eq!(
        tokenize_lang("Running quickly", "english").unwrap(),
        vec!["run", "quick"]
    );
}

#[test]
fn stem_english() {
    assert_eq!(stem("running", "english").unwrap(), "run");
    assert_eq!(stem("dogs", "english").unwrap(), "dog");
}

#[test]
fn supported_languages_includes_english() {
    assert!(SUPPORTED_LANGUAGES.contains(&"english"));
    assert!(SUPPORTED_LANGUAGES.len() >= 10);
}

#[test]
fn version_string_present() {
    assert!(tantivy_version().contains("tantivy"));
}

#[test]
fn unknown_language_and_bad_json_are_errors() {
    assert!(matches!(
        stem("x", "klingon"),
        Err(SearchError::UnknownLanguage(_))
    ));
    assert!(matches!(
        parse_docs("not json"),
        Err(SearchError::BadDocsJson(_))
    ));
}

#[test]
fn garbage_and_empty_never_panic() {
    let _ = bm25_search(&corpus(), "!@#$%^&*", "english", 10);
    let _ = tokenize_default("");
    let _ = parse_docs("");
    assert!(tokenize_default("").unwrap().is_empty());
}
