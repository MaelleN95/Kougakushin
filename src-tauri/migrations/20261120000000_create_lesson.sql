-- Migration : création de la table des leçons (voir SPEC.md, section 3.1 et 5.1).

CREATE TABLE lesson (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    date TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);