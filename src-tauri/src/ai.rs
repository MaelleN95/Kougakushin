// Fonctionnalité "IA" : dialogue avec Ollama (modèle local).

mod client;
mod commands;

// Client partagé, enregistré au démarrage dans lib.rs.
pub(crate) use client::Ollama;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::check_ollama;
