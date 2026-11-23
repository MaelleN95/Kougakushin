// Déclaration des modules : chaque `mod x;` correspond au fichier src/x.rs (comme un namespace PHP).
mod ai;
mod cards;
mod db;
mod error;
mod items;
mod lessons;
mod settings;

use ai::{
    check_ollama, correct_translation, end_conversation, generate_translation,
    list_conversation_themes, reply_conversation, start_conversation, translate_message,
};
use cards::{get_review_session, review_card};
use items::{create_item, delete_item, list_items, set_item_active, update_item};
use lessons::{create_lesson, delete_lesson, list_lessons, update_lesson};
use settings::{get_settings, update_settings};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Journal : terminal en développement + fichier dans %LOCALAPPDATA%\com.kougakushin.app\logs\.
        // Info et plus grave seulement (sqlx écrit chaque requête au niveau Debug).
        // Fichier de 1 Mo au plus, 5 anciens fichiers gardés.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        // Exécuté une fois au démarrage, avant l'ouverture de la fenêtre (les plugins sont déjà prêts).
        .setup(|app| {
            // `setup` n'est pas async : block_on attend ici la fin de l'initialisation de la base.
            let pool = tauri::async_runtime::block_on(db::init(app.handle())).map_err(|e| {
                log::error!("Initialisation de la base impossible : {e}");
                e
            })?;
            // Le pool devient un "état" partagé, injectable dans n'importe quelle commande
            // (équivalent de l'injection d'un service dans un contrôleur Symfony).
            app.manage(pool);
            // Client HTTP vers Ollama, injectable dans les commandes comme le pool.
            app.manage(ai::Ollama::new()?);
            Ok(())
        })
        // Les commandes Tauri sont des fonctions Rust exposées à l'interface JavaScript.
        .invoke_handler(tauri::generate_handler![
            analyser_mot,
            list_lessons,
            create_lesson,
            update_lesson,
            delete_lesson,
            list_items,
            create_item,
            update_item,
            set_item_active,
            delete_item,
            get_review_session,
            review_card,
            get_settings,
            update_settings,
            check_ollama,
            generate_translation,
            correct_translation,
            list_conversation_themes,
            start_conversation,
            reply_conversation,
            end_conversation,
            translate_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Ici, je décris à quoi doit ressembler ma sortie de fonction. "Struct" est un peu comme une déclaration de classe en PHP
// "derive" génère du code automatiquement pour moi, ici c'est pour la sérialisation de l'objet en JSON
#[derive(serde::Serialize)]
struct AnalyseDuMot {
    octets: usize,
    caracteres: usize,
}

// Je déclare la fonction "analyser_mot" ici
// Mon paramètre est une string que je reçois et que j'analyse donc comme je l'emprunte (je ne la prends pas possession, je n'ai pas besoin de la modifier), elle est déclarée en &str plutôt que String.
// Je n'ai pas besoin de mettre de "return" pour faire un retour de fonction en Rust
#[tauri::command]
fn analyser_mot(mot: &str) -> AnalyseDuMot {
    let octets: usize = mot.len();
    let caracteres: usize = mot.chars().count();

    AnalyseDuMot { octets, caracteres }
}
