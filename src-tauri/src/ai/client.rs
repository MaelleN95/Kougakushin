// Client HTTP minimal vers l'API locale d'Ollama (https://docs.ollama.com/api).

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::AppError;

const DEFAULT_BASE_URL: &str = "http://localhost:11434";

// Partagé par toutes les commandes via `app.manage` (comme le pool SQLite).
// reqwest::Client garde ses connexions ouvertes : on en crée un seul pour toute l'application.
pub(crate) struct Ollama {
    http: reqwest::Client,
    base_url: String,
}

// Formes des réponses JSON d'Ollama : seuls les champs utiles sont déclarés, serde ignore le reste.
#[derive(serde::Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(serde::Deserialize)]
struct TagsResponse {
    models: Vec<ModelEntry>,
}

#[derive(serde::Deserialize)]
struct ModelEntry {
    name: String,
}

#[derive(serde::Deserialize)]
struct ErrorResponse {
    error: String,
}

impl Ollama {
    pub(crate) fn new() -> Result<Ollama, AppError> {
        Ollama::with_base_url(DEFAULT_BASE_URL)
    }

    // Adresse modifiable pour les tests.
    fn with_base_url(base_url: &str) -> Result<Ollama, AppError> {
        let http = reqwest::Client::builder()
            // Ollama est local : s'il ne répond pas en 3 s, il n'est pas lancé.
            .connect_timeout(Duration::from_secs(3))
            // Le premier appel charge le modèle en mémoire (environ 25 s mesurées) : marge large.
            .timeout(Duration::from_secs(180))
            .build()?;
        Ok(Ollama {
            http,
            base_url: base_url.to_string(),
        })
    }

    // GET générique : `T` est le type attendu, déduit de l'appelant.
    // DeserializeOwned : T se construit depuis le JSON sans emprunter la réponse.
    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        let response = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .send()
            .await?;
        read_json(response).await
    }

    // Version d'Ollama : sert aussi de test de connexion.
    pub(crate) async fn version(&self) -> Result<String, AppError> {
        let body: VersionResponse = self.get_json("/api/version").await?;
        Ok(body.version)
    }

    // Noms des modèles téléchargés (ex. "gemma4:e4b-it-q4_K_M").
    pub(crate) async fn installed_models(&self) -> Result<Vec<String>, AppError> {
        let body: TagsResponse = self.get_json("/api/tags").await?;
        Ok(body.models.into_iter().map(|m| m.name).collect())
    }
}

// Réponse HTTP => T. Si Ollama renvoie une erreur (ex. modèle absent), on garde son message.
async fn read_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, AppError> {
    let status = response.status();
    if !status.is_success() {
        let message = match response.json::<ErrorResponse>().await {
            Ok(body) => body.error,
            Err(_) => status.to_string(),
        };
        return Err(AppError::OllamaRefused(message));
    }
    Ok(response.json::<T>().await?)
}

#[cfg(test)]
mod tests {
    use super::Ollama;

    #[test]
    fn ollama_absent_donne_un_message_clair() {
        tauri::async_runtime::block_on(async {
            // Port 9 : aucun service n'y écoute.
            let ollama = Ollama::with_base_url("http://127.0.0.1:9").unwrap();
            let error = ollama.version().await.unwrap_err();
            let message = serde_json::to_string(&error).unwrap();
            assert!(
                message.starts_with("\"Ollama"),
                "message inattendu : {message}"
            );
        });
    }

    // Nécessite Ollama lancé : `cargo test -- --ignored ollama_reel`.
    #[test]
    #[ignore]
    fn ollama_reel() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            assert!(!ollama.version().await.unwrap().is_empty());
            println!("modèles : {:?}", ollama.installed_models().await.unwrap());
        });
    }
}
