//! Composed example queries — each combines two or more of this worker's own
//! objects in one realistic query. Defined once here and referenced from both
//! the schema-level `vgi.example_queries` list (`lib.rs`) and the example list
//! of the object that drives the query, so the SQL is never copy-pasted.
//! Keep each SQL on one line (it is embedded in JSON tags).

/// `tokenize` → `stem`: group a text's word variants under their English root.
pub(crate) const TOKENIZE_GROUP_BY_ROOT: (&str, &str) = (
    "Tokenize a sentence (no stemming), then stem each token with stem() to group the word variants under their English Snowball root, most frequent root first.",
    "SELECT tantivy.main.stem(tok, 'english') AS root, list(DISTINCT tok ORDER BY tok) AS variants, count(*) AS occurrences FROM unnest(tantivy.main.tokenize('Runners were running; the runner runs and ran a run.')) AS u(tok) GROUP BY root ORDER BY occurrences DESC, root",
);

/// `supported_languages` → `stem`: apply every stemmer to one word.
pub(crate) const STEM_EVERY_LANGUAGE: (&str, &str) = (
    "Stem the word 'connections' with every supported Snowball stemmer (one row per language from supported_languages) and keep only the languages whose stemmer changes it.",
    "SELECT lang, tantivy.main.stem('connections', lang) AS root FROM tantivy.main.supported_languages WHERE tantivy.main.stem('connections', lang) <> 'connections' ORDER BY lang",
);

/// `bm25_search` → join back to the source rows → `tokenize`: explain each hit.
pub(crate) const SEARCH_EXPLAIN_HITS: (&str, &str) = (
    "Rank product titles for 'running OR shoes' with bm25_search, join the hits back to the titles by id, and use tokenize(…, 'english') to list which stemmed query terms each hit contains.",
    r#"SELECT p.title, round(h.score, 3) AS score, list_sort(list_intersect(tantivy.main.tokenize(p.title, 'english'), tantivy.main.tokenize('running shoes', 'english'))) AS matched_terms FROM tantivy.main.bm25_search('[{"id":1,"text":"Trail running shoes"},{"id":2,"text":"Leather dress shoes"},{"id":3,"text":"Runners guide to the marathon"},{"id":4,"text":"Cast iron skillet"},{"id":5,"text":"Running socks"}]', 'running OR shoes') AS h JOIN (VALUES (1, 'Trail running shoes'), (2, 'Leather dress shoes'), (3, 'Runners guide to the marathon'), (4, 'Cast iron skillet'), (5, 'Running socks')) AS p(id, title) ON p.id = h.doc_id ORDER BY h.score DESC, p.title"#,
);
