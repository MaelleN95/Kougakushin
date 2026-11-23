// Fonctionnalité "réglages" : préférences de l'utilisatrice, stockées en clé / valeur.

mod commands;
mod model;
mod repository;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::{get_settings, update_settings};

// Lu par `cards` (nombre de nouvelles cartes par jour).
pub(crate) use commands::load;
