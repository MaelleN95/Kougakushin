// Déclaration des modules : chaque `mod x;` correspond au fichier src/x.rs (comme un namespace PHP).
mod db;
mod lessons;

use lessons::list_lessons;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Exécuté une fois au démarrage, avant l'ouverture de la fenêtre.
        .setup(|app| {
            // `setup` n'est pas async : block_on attend ici la fin de l'initialisation de la base.
            let pool = tauri::async_runtime::block_on(db::init(app.handle()))?;
            // Le pool devient un « état » partagé, injectable dans n'importe quelle commande
            // (équivalent de l'injection d'un service dans un contrôleur Symfony).
            app.manage(pool);
            Ok(())
        })
        // Les commandes Tauri sont des fonctions Rust exposées à l'interface JavaScript.
        .invoke_handler(tauri::generate_handler![analyser_mot, list_lessons])
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
