-- Migration : cartes de conjugaison (D40, D41).
-- 1. Table des formes verbales connues (interrupteur "je connais / pas encore").
-- 2. Table card reconstruite : plus de CHECK sur direction (l'enum Rust Direction valide les
--    valeurs), nouvelle colonne `form`, unicité (item_id, direction, form).
-- 3. Cartes de la forme ます, connue par défaut, pour les verbes déjà saisis.
--
-- SQLite ne sait pas modifier une contrainte : on crée la nouvelle table, on copie, on supprime
-- l'ancienne, on renomme. Les id, l'état FSRS et l'historique review_log sont conservés.
-- sqlx exécute la migration dans une transaction, où PRAGMA foreign_keys ne peut pas changer :
-- supprimer `card` supprime donc en cascade review_log. On le met de côté et on le remet après.

-- --- 1. Formes connues ---

CREATE TABLE conjugation_form (
    form TEXT PRIMARY KEY, -- valeur de VerbForm::as_str en Rust (pas de CHECK, comme direction)
    known INTEGER NOT NULL DEFAULT 0 CHECK (known IN (0, 1))
);

-- ます connue par défaut : c'est la forme polie déjà autorisée à l'IA (D11) et la première
-- enseignée. Les autres sont activées dans les Réglages au fil des cours.
INSERT INTO conjugation_form (form, known) VALUES
    ('dictionary', 0),
    ('masu', 1),
    ('masen', 0),
    ('mashita', 0),
    ('masen_deshita', 0),
    ('te', 0),
    ('ta', 0),
    ('nai', 0),
    ('nakatta', 0);

-- --- 2. Reconstruction de card ---

CREATE TABLE card_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id INTEGER NOT NULL REFERENCES item (id) ON DELETE CASCADE,
    direction TEXT NOT NULL, -- 'fr_jp', 'jp_fr', 'kanji_read', 'conjugation' (enum Rust Direction)
    form TEXT, -- forme de conjugaison (VerbForm), seulement pour direction = 'conjugation'

    -- État FSRS (champs de rs_fsrs::Card), inchangé.
    next_review_at TEXT NOT NULL DEFAULT (datetime('now')),
    stability REAL NOT NULL DEFAULT 0,
    difficulty REAL NOT NULL DEFAULT 0,
    elapsed_days INTEGER NOT NULL DEFAULT 0,
    scheduled_days INTEGER NOT NULL DEFAULT 0,
    reps INTEGER NOT NULL DEFAULT 0,
    lapses INTEGER NOT NULL DEFAULT 0,
    state INTEGER NOT NULL DEFAULT 0 CHECK (state BETWEEN 0 AND 3), -- 0 = nouvelle
    last_review TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Une forme pour les cartes de conjugaison, aucune pour les autres.
    CHECK ((direction = 'conjugation') = (form IS NOT NULL))
);

INSERT INTO card_new (id, item_id, direction, form, next_review_at, stability, difficulty,
                      elapsed_days, scheduled_days, reps, lapses, state, last_review, created_at)
SELECT id, item_id, direction, NULL, next_review_at, stability, difficulty,
       elapsed_days, scheduled_days, reps, lapses, state, last_review, created_at
FROM card;

-- Compteur AUTOINCREMENT : on garde celui de l'ancienne table, pour ne jamais réutiliser l'id
-- d'une carte supprimée. sqlite_sequence n'a pas de clé : mise à jour, ou ajout si absent.
UPDATE sqlite_sequence
SET seq = (SELECT seq FROM sqlite_sequence WHERE name = 'card')
WHERE name = 'card_new' AND EXISTS (SELECT 1 FROM sqlite_sequence WHERE name = 'card');
INSERT INTO sqlite_sequence (name, seq)
SELECT 'card_new', seq FROM sqlite_sequence
WHERE name = 'card' AND NOT EXISTS (SELECT 1 FROM sqlite_sequence WHERE name = 'card_new');

-- Historique mis de côté, puis vidé explicitement : qu'il y ait cascade (foreign_keys activé)
-- ou non, la table est vide avant d'être remplie de nouveau.
CREATE TEMP TABLE review_log_backup AS SELECT * FROM review_log;
DELETE FROM review_log;

DROP TABLE card;
-- review_log référence "card" par son nom : après le renommage, il pointe vers la nouvelle table.
ALTER TABLE card_new RENAME TO card;

INSERT INTO review_log (id, card_id, rating, state, elapsed_days, scheduled_days, reviewed_at)
SELECT id, card_id, rating, state, elapsed_days, scheduled_days, reviewed_at
FROM review_log_backup;
DROP TABLE review_log_backup;

CREATE INDEX idx_card_next_review_at ON card (next_review_at);
-- Une seule carte par sens (et par forme) pour un même élément. ifnull : en SQLite, deux NULL
-- ne sont jamais égaux dans un UNIQUE ; sans lui, deux cartes fr_jp du même élément passeraient.
CREATE UNIQUE INDEX idx_card_item_direction_form ON card (item_id, direction, ifnull(form, ''));

-- --- 3. Cartes ます des verbes déjà saisis ---
-- Même règle de validité que conjugation::conjugate en Rust (terminaison compatible avec le type).

INSERT INTO card (item_id, direction, form)
SELECT id, 'conjugation', 'masu' FROM item
WHERE kind = 'vocab'
  AND length(trim(kana)) >= 2
  AND (
    (word_type = 'ru_verb' AND trim(kana) LIKE '%る')
    OR (word_type = 'u_verb'
        AND substr(trim(kana), -1) IN ('う', 'つ', 'る', 'む', 'ぶ', 'ぬ', 'く', 'ぐ', 'す'))
    OR (word_type = 'irregular_verb'
        AND (trim(kana) LIKE '%する' OR trim(kana) LIKE '%くる'))
  );
