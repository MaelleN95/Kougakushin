// Fonctionnalité "IA" : dialogue avec Ollama (modèle local).

mod client;
mod commands;
mod conversation;
mod knowledge_prompt;
mod translation;

// Client partagé, enregistré au démarrage dans lib.rs.
pub(crate) use client::Ollama;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::check_ollama;
pub(crate) use conversation::{
    end_conversation, list_conversation_themes, reply_conversation, start_conversation,
    translate_message,
};
pub(crate) use translation::{correct_translation, generate_translation};
