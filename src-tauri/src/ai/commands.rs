// Commandes Tauri liées à l'IA.

use sqlx::SqlitePool;
use tauri::State;

use super::client::Ollama;
use crate::error::AppError;
use crate::settings;

// État d'Ollama pour l'écran Réglages.
#[derive(Debug, serde::Serialize)]
pub(crate) struct OllamaStatus {
    version: String,
    models: Vec<String>,
    // Le modèle choisi dans les réglages est-il téléchargé ?
    configured_model_installed: bool,
}

// "Tester la connexion" : Ollama répond-il, et avec quels modèles ?
#[tauri::command]
pub(crate) async fn check_ollama(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
) -> Result<OllamaStatus, AppError> {
    let version = ollama.version().await?;
    let models = ollama.installed_models().await?;
    let configured = settings::load(pool.inner()).await?.ollama_model;
    Ok(OllamaStatus {
        version,
        configured_model_installed: models.contains(&configured),
        models,
    })
}
