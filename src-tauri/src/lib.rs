#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // J'enregistre la fonction "analyser_mot" ici
        .invoke_handler(tauri::generate_handler![analyser_mot])
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
