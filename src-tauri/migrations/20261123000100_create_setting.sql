-- Migration : réglages de l'application, en clé / valeur (voir SPEC.md, sections 3.6 et 5.1).
-- Pas de valeurs insérées : les valeurs par défaut sont dans le code Rust (settings/model.rs).

CREATE TABLE setting (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
