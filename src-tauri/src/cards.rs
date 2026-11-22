// Fonctionnalité "cartes" : flashcards générées depuis le vocabulaire, planifiées par FSRS.

mod model;
mod repository;

// Appelé par `items` à chaque création ou modification d'élément.
pub(crate) use repository::sync_for_item;
