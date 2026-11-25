// Fonctionnalité "éléments" : vocabulaire et points de grammaire rattachés à une leçon.

mod commands;
mod knowledge;
mod model;
mod repository;

// Seules les commandes sortent du module, pour `generate_handler!` dans lib.rs.
pub(crate) use commands::{
    create_item, delete_item, delete_items, list_items, move_items, set_item_active,
    set_items_active, update_item,
};

// Lu par `cards` pour savoir quelles cartes générer.
pub(crate) use model::ItemContent;

// Lu par `ai` pour les exercices (vocabulaire et grammaire actifs).
pub(crate) use knowledge::{Knowledge, find_knowledge};

// Construits à la main dans les tests de `ai`.
#[cfg(test)]
pub(crate) use knowledge::{KnownGrammar, KnownWord};
