// Fonctionnalité "IA" : dialogue avec Ollama (modèle local).

mod client;
mod commands;
mod knowledge_prompt;
mod translation;

// Client partagé, enregistré au démarrage dans lib.rs.
pub(crate) use client::Ollama;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::check_ollama;
pub(crate) use translation::{correct_translation, generate_translation};
