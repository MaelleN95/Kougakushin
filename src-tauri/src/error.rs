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
}

// Tauri exige que l'erreur d'une commande soit sérialisable.
// Le front reçoit une chaîne ; le détail technique d'une erreur SQL reste côté Rust.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            AppError::Database(_) => {
                // Détail visible dans le terminal de `npm run tauri dev` uniquement.
                eprintln!("[AppError] {self}");
                serializer.serialize_str("Erreur de base de données")
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
