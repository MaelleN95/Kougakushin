// Fonctionnalité "sauvegarde" : export et import JSON de toutes les données (SPEC.md 3.6).
// Décisions D23 (l'import remplace tout, après une sauvegarde automatique) et D24 (tout est
// exporté, progression FSRS et historique compris).

mod commands;
mod model;
mod repository;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::{export_data, import_data};
