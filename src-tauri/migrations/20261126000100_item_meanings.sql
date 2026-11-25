-- Migration : les sens français d'un mot deviennent une vraie liste (D43 B).
--
-- Une ligne par sens, dans l'ordre de saisie. La colonne item.meaning est gardée : la retirer
-- obligerait à reconstruire la table item (son CHECK exige meaning pour le vocabulaire), et
-- DROP TABLE item supprimerait en CASCADE les cartes et l'historique. Elle devient un texte
-- dérivé ("je; moi"), réécrit par Rust à chaque enregistrement et jamais relu par l'application.

CREATE TABLE item_meaning (
    -- CASCADE : supprimer un élément supprime ses sens.
    item_id INTEGER NOT NULL REFERENCES item (id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0), -- 0 = premier sens
    meaning TEXT NOT NULL CHECK (trim(meaning) <> ''),
    -- La clé primaire sert aussi d'index pour lire les sens d'un élément.
    PRIMARY KEY (item_id, position)
);

-- Conversion des mots existants : découpe sur ";", espaces retirés, sens vides et doublons ignorés.
-- `split` avance d'un morceau à chaque tour : `piece` = morceau lu, `rest` = ce qui reste à lire.
INSERT INTO item_meaning (item_id, position, meaning)
WITH RECURSIVE split (item_id, position, piece, rest) AS (
    SELECT id, 0, '', meaning || ';' FROM item WHERE kind = 'vocab'
    UNION ALL
    SELECT item_id, position + 1,
           substr(rest, 1, instr(rest, ';') - 1),
           substr(rest, instr(rest, ';') + 1)
    FROM split
    WHERE rest <> ''
),
-- Espaces, tabulations et retours à la ligne retirés aux deux bouts.
cleaned AS (
    SELECT item_id, MIN(position) AS first_position,
           trim(piece, char(32, 9, 10, 13)) AS meaning
    FROM split
    WHERE position > 0 AND trim(piece, char(32, 9, 10, 13)) <> ''
    GROUP BY item_id, trim(piece, char(32, 9, 10, 13))
)
-- Positions renumérotées 0, 1, 2... sans trou.
SELECT item_id, row_number() OVER (PARTITION BY item_id ORDER BY first_position) - 1, meaning
FROM cleaned;

-- Filet de sécurité : un mot dont la traduction ne contenait que des ";" ou des espaces garde
-- un sens, pour ne pas devenir illisible (un mot sans sens est refusé à la lecture).
INSERT INTO item_meaning (item_id, position, meaning)
SELECT id, 0, CASE WHEN trim(meaning) <> '' THEN meaning ELSE '(sans traduction)' END
FROM item
WHERE kind = 'vocab' AND id NOT IN (SELECT item_id FROM item_meaning);

-- Texte dérivé remis au format écrit par Rust ("je; moi").
UPDATE item
SET meaning = (SELECT group_concat(m.meaning, '; ' ORDER BY m.position)
               FROM item_meaning m WHERE m.item_id = item.id)
WHERE kind = 'vocab';
