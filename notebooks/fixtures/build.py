"""Regenerate the authored search corpus; no external document content is used."""

from pathlib import Path
import hashlib
import json


def main() -> None:
    """Write ten fixed input records and their provenance hash."""
    texts = [
        "Battery storage for solar microgrids",
        "Batteries store energy in modular storage systems",
        "Storage maintenance for battery banks and battery enclosures",
        "Battery maintenance and charging checks",
        "Cold storage warehouse temperature checks",
        "Archival storage and document retention",
        "Solar panels and wind turbines",
        "Battery battery battery overview",
        "",
        None,
    ]
    root = Path(__file__).resolve().parent
    text = json.dumps([dict(doc_id=i + 101, text=t) for i, t in enumerate(texts)], indent=2) + "\n"
    (root / "corpus.json").write_text(text)
    (root / "manifest.json").write_text(
        json.dumps(
            {
                "note": "Authored short document strings for a retrieval demonstration, not product specifications or external research.",
                "sha256": {"corpus.json": hashlib.sha256(text.encode()).hexdigest()},
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
