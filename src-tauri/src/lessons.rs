// Fonctionnalité "leçons" : modèle, accès SQL et commandes Tauri.

mod commands;
mod model;
mod repository;

// Seules les commandes sortent du module, pour `generate_handler!` dans lib.rs.
pub(crate) use commands::{create_lesson, delete_lesson, list_lessons, update_lesson};
