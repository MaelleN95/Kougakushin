// Erreurs de l'application, communes à toutes les fonctionnalités.

// thiserror génère `Display` (le message) à partir des #[error(...)].
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    // #[from] : le `?` convertit seul un sqlx::Error en AppError::Database.
    #[error("Erreur de base de données : {0}")]
    Database(#[from] sqlx::Error),

    // Le paramètre est le nom de l'élément : NotFound("Leçon") => "Leçon introuvable".
    #[error("{0} introuvable")]
    NotFound(&'static str),

    // Donnée refusée avant toute écriture en base.
    #[error("{0}")]
    Validation(&'static str),

    // Action impossible dans l'état actuel (ex. supprimer une leçon non vide).
    #[error("{0}")]
    Conflict(&'static str),

    // Ligne en base qui ne respecte pas le modèle Rust ; le paramètre nomme le champ fautif.
    #[error("Donnée incohérente en base : {0}")]
    InvalidData(&'static str),

    // Échec d'un appel HTTP à Ollama (pas lancé, trop lent, réponse illisible...).
    #[error("Erreur de communication avec Ollama : {0}")]
    Ollama(#[from] reqwest::Error),

    // Ollama a répondu par une erreur (ex. modèle non installé) ; message d'Ollama inclus.
    #[error("Ollama a refusé la demande : {0}")]
    OllamaRefused(String),

    // Le modèle a répondu, mais pas dans la forme JSON demandée.
    #[error("Réponse de l'IA illisible. Réessaie.")]
    InvalidAiResponse,

    // Lecture ou écriture d'un fichier (sauvegarde) impossible.
    #[error("Erreur de fichier : {0}")]
    File(#[from] std::io::Error),

    // Fichier de sauvegarde illisible ou incohérent ; rien n'a été modifié.
    #[error("Fichier de sauvegarde invalide : {0}")]
    InvalidBackup(String),
}

// Vrai si la base a refusé l'opération à cause d'une clé étrangère
// (id de leçon inexistant à l'insertion, ou leçon encore référencée à la suppression).
pub(crate) fn is_foreign_key_violation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => db_error.is_foreign_key_violation(),
        _ => false,
    }
}

// Message pour l'utilisatrice selon la cause d'une erreur HTTP vers Ollama.
fn ollama_message(error: &reqwest::Error) -> &'static str {
    if error.is_connect() {
        "Ollama ne répond pas. Vérifie qu'il est lancé (icône dans la barre des tâches)."
    } else if error.is_timeout() {
        "Ollama met trop de temps à répondre. Réessaie dans un instant."
    } else if error.is_decode() {
        "Réponse de l'IA illisible. Réessaie."
    } else {
        "Erreur de communication avec Ollama."
    }
}

// Tauri exige que l'erreur d'une commande soit sérialisable.
// Le front reçoit une chaîne ; le détail technique d'une erreur SQL va dans le journal.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            AppError::Database(_) => {
                // Détail écrit dans le journal (terminal en dev + fichier de log).
                log::error!("{self}");
                serializer.serialize_str("Erreur de base de données")
            }
            // Message compréhensible selon la cause ; le détail technique va dans le journal.
            AppError::Ollama(error) => {
                log::error!("{self}");
                serializer.serialize_str(ollama_message(error))
            }
            AppError::OllamaRefused(_) => {
                log::warn!("{self}");
                serializer.serialize_str(&self.to_string())
            }
            _ => serializer.serialize_str(&self.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn masque_le_detail_sql_au_front() {
        let json = serde_json::to_string(&AppError::Database(sqlx::Error::RowNotFound));
        assert_eq!(json.ok().as_deref(), Some("\"Erreur de base de données\""));
    }

    #[test]
    fn transmet_les_messages_metier() {
        let json = serde_json::to_string(&AppError::NotFound("Leçon"));
        assert_eq!(json.ok().as_deref(), Some("\"Leçon introuvable\""));
    }
}
