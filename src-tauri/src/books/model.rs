// Structures de données des manuels.

use crate::error::AppError;

// Manuel lu en base et renvoyé au front.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub(crate) struct Book {
    id: i64,
    title: String,
    created_at: String,
}

// Données envoyées par le front pour créer ou renommer un manuel.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BookInput {
    pub(super) title: String,
}

impl BookInput {
    // Titre sans espaces autour ; refusé s'il est vide.
    pub(super) fn validated_title(&self) -> Result<&str, AppError> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err(AppError::Validation(
                "Le titre du manuel ne peut pas être vide",
            ));
        }
        Ok(title)
    }
}
