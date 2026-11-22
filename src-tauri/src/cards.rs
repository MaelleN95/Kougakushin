// Fonctionnalité "cartes" : flashcards générées depuis le vocabulaire, planifiées par FSRS.

mod commands;
mod model;
mod repository;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::{get_review_session, review_card};

// Appelé par `items` à chaque création ou modification d'élément.
pub(crate) use repository::sync_for_item;
