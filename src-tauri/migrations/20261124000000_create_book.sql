-- Migration : manuels (ex. "Marugoto A1"), qui regroupent des leçons (D29).

CREATE TABLE book (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- NOCASE : "marugoto a1" et "Marugoto A1" sont le même manuel.
    title TEXT NOT NULL UNIQUE COLLATE NOCASE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- NULL = leçon sans manuel (cours libre).
-- Action par défaut (NO ACTION) : impossible de supprimer un manuel qui contient des leçons.
ALTER TABLE lesson ADD COLUMN book_id INTEGER REFERENCES book (id);

CREATE INDEX idx_lesson_book_id ON lesson (book_id);
