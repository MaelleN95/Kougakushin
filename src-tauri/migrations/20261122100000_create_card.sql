-- Migration : cartes de révision FSRS et historique des révisions (voir SPEC.md, sections 3.3 et 5.1).
-- Dates : texte UTC au format de datetime('now') ("AAAA-MM-JJ HH:MM:SS"), comparable en tant que texte.

CREATE TABLE card (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- CASCADE : supprimer un élément supprime ses cartes (et, en chaîne, leur historique).
    -- Pour garder un élément sans le réviser, on le désactive.
    item_id INTEGER NOT NULL REFERENCES item (id) ON DELETE CASCADE,
    direction TEXT NOT NULL CHECK (direction IN ('fr_jp', 'jp_fr', 'kanji_read')),

    -- État FSRS (champs de rs_fsrs::Card).
    due TEXT NOT NULL DEFAULT (datetime('now')),
    stability REAL NOT NULL DEFAULT 0,
    difficulty REAL NOT NULL DEFAULT 0,
    elapsed_days INTEGER NOT NULL DEFAULT 0,
    scheduled_days INTEGER NOT NULL DEFAULT 0,
    reps INTEGER NOT NULL DEFAULT 0,
    lapses INTEGER NOT NULL DEFAULT 0,
    state INTEGER NOT NULL DEFAULT 0 CHECK (state BETWEEN 0 AND 3), -- 0 = nouvelle
    last_review TEXT, -- NULL tant que la carte n'a jamais été révisée

    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Une seule carte par sens pour un même élément.
    UNIQUE (item_id, direction)
);

CREATE INDEX idx_card_due ON card (due);

CREATE TABLE review_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id INTEGER NOT NULL REFERENCES card (id) ON DELETE CASCADE,
    rating INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 4), -- 1 À revoir ... 4 Facile
    state INTEGER NOT NULL CHECK (state BETWEEN 0 AND 3), -- état de la carte avant la révision
    elapsed_days INTEGER NOT NULL,
    scheduled_days INTEGER NOT NULL,
    reviewed_at TEXT NOT NULL
);

CREATE INDEX idx_review_log_card_id ON review_log (card_id);

-- Cartes des éléments déjà saisis (même règle que cards::directions_for en Rust).
INSERT INTO card (item_id, direction)
SELECT id, 'fr_jp' FROM item WHERE kind = 'vocab';

INSERT INTO card (item_id, direction)
SELECT id, 'jp_fr' FROM item WHERE kind = 'vocab';

INSERT INTO card (item_id, direction)
SELECT id, 'kanji_read' FROM item
WHERE kind = 'vocab' AND kanji IS NOT NULL AND trim(kanji) <> '';
