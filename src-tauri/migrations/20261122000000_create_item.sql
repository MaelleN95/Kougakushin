-- Migration : éléments à apprendre, vocabulaire et grammaire (voir SPEC.md, sections 3.2 et 5.1).

CREATE TABLE item (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Action par défaut (NO ACTION) : impossible de supprimer une leçon qui contient des éléments.
    -- Pas de RESTRICT : SQLite le signale avec un autre code d'erreur, que sqlx ne reconnaît pas
    -- comme une violation de clé étrangère.
    lesson_id INTEGER NOT NULL REFERENCES lesson (id),
    kind TEXT NOT NULL CHECK (kind IN ('vocab', 'grammar')),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),

    -- Vocabulaire
    kana TEXT,
    romaji TEXT,
    kanji TEXT,
    meaning TEXT,
    word_type TEXT CHECK (word_type IN (
        'noun', 'ru_verb', 'u_verb', 'irregular_verb', 'i_adjective', 'na_adjective', 'other'
    )),

    -- Grammaire
    title TEXT,
    explanation TEXT,

    -- Commun
    example_jp TEXT,
    example_fr TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Chaque type a ses champs obligatoires, et aucun champ de l'autre type.
    CHECK (
        (kind = 'vocab'
            AND kana IS NOT NULL AND meaning IS NOT NULL
            AND title IS NULL AND explanation IS NULL)
        OR
        (kind = 'grammar'
            AND title IS NOT NULL AND explanation IS NOT NULL
            AND kana IS NULL AND romaji IS NULL AND kanji IS NULL
            AND meaning IS NULL AND word_type IS NULL)
    )
);

-- Accélère le filtre par leçon.
CREATE INDEX idx_item_lesson_id ON item (lesson_id);
