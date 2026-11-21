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
// On envoie le message seul : le front reçoit une chaîne, comme avant.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
