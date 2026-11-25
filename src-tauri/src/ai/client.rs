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

#[derive(serde::Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(serde::Deserialize)]
struct ChatMessage {
    content: String,
}

// Auteur d'un message : l'élève (user) ou l'IA (assistant). Noms imposés par Ollama.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    User,
    Assistant,
}

// Un message d'une conversation (aussi reçu du front, d'où deny_unknown_fields).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChatTurn {
    pub(crate) role: Role,
    pub(crate) content: String,
}

impl ChatTurn {
    pub(crate) fn user(content: impl Into<String>) -> ChatTurn {
        ChatTurn {
            role: Role::User,
            content: content.into(),
        }
    }
}

// Une demande à l'IA : consignes (system), échanges (turns), forme JSON imposée.
pub(crate) struct ChatRequest<'a> {
    pub(crate) model: &'a str,
    pub(crate) system: &'a str,
    pub(crate) turns: &'a [ChatTurn], // dans l'ordre ; le dernier est la demande en cours
    pub(crate) schema: serde_json::Value, // schéma JSON que la réponse doit respecter
    pub(crate) temperature: f64,      // 0 = déterministe, 1 = très varié
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

    // Traduction japonais => français par le modèle dédié (TranslateGemma, décision D22).
    // Format de consigne imposé par la page du modèle sur ollama.com (deux lignes vides avant
    // le texte). `num_gpu: 0` : il tourne sur le processeur, pour laisser la carte graphique au
    // modèle principal ; sinon Ollama les échange à chaque appel (+16 s mesurées).
    // `glossary` ("ねこ : chat ; マエル : Maëlle", vide = aucun) : sens des mots de l'élève.
    // Mesure du 2026-11-25 : sans lui, les noms propres en katakana sont devinés
    // (ラブヤディス => "Lovey Lady", マエル => "Mael") ; avec, ils sont justes.
    pub(crate) async fn translate_ja_to_fr(
        &self,
        model: &str,
        text: &str,
        glossary: &str,
    ) -> Result<String, AppError> {
        let glossary = if glossary.is_empty() {
            String::new()
        } else {
            format!(" Use this glossary: {glossary}.")
        };
        let content = format!(
            "You are a professional Japanese (ja) to French (fr) translator. Your goal is to \
             accurately convey the meaning and nuances of the original Japanese text while \
             adhering to French grammar, vocabulary, and cultural sensitivities.\n\
             Produce only the French translation, without any additional explanations or \
             commentary.{glossary} Please translate the following Japanese text into \
             French:\n\n\n{text}"
        );
        let body = serde_json::json!({
            "model": model,
            "stream": false,
            "options": { "temperature": 0, "num_gpu": 0 },
            "messages": [{ "role": "user", "content": content }],
        });
        let response = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;
        let chat: ChatResponse = read_json(response).await?;
        Ok(chat.message.content.trim().to_string())
    }

    // Envoie une demande et lit la réponse comme un `T`.
    // `format` : Ollama contraint le modèle à produire un JSON conforme au schéma.
    // `think: false` : pas de phase de "réflexion" affichée, réponse plus rapide.
    pub(crate) async fn chat_json<T: DeserializeOwned>(
        &self,
        request: ChatRequest<'_>,
    ) -> Result<T, AppError> {
        // Consignes d'abord, puis les échanges dans l'ordre.
        let mut messages = vec![serde_json::json!({ "role": "system", "content": request.system })];
        for turn in request.turns {
            messages.push(serde_json::json!({ "role": turn.role, "content": turn.content }));
        }
        let body = serde_json::json!({
            "model": request.model,
            "stream": false,
            "think": false,
            "format": request.schema,
            "options": { "temperature": request.temperature },
            "messages": messages,
        });
        let response = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;
        let chat: ChatResponse = read_json(response).await?;

        // Le schéma est imposé, mais on revérifie : un modèle peut quand même dévier.
        serde_json::from_str(&chat.message.content).map_err(|e| {
            log::warn!("Réponse de l'IA illisible ({e}) : {}", chat.message.content);
            AppError::InvalidAiResponse
        })
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
