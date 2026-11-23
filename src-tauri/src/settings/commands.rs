// Commandes Tauri des réglages.

use sqlx::SqlitePool;
use tauri::State;

use super::model::Settings;
use super::repository;
use crate::error::AppError;

// Réglages actuels (valeurs par défaut pour ce qui n'a jamais été modifié).
pub(crate) async fn load(pool: &SqlitePool) -> Result<Settings, AppError> {
    Settings::from_pairs(repository::find_all(pool).await?)
}

#[tauri::command]
pub(crate) async fn get_settings(pool: State<'_, SqlitePool>) -> Result<Settings, AppError> {
    load(pool.inner()).await
}

// Enregistre tous les réglages d'un coup (transaction : tout ou rien).
#[tauri::command]
pub(crate) async fn update_settings(
    pool: State<'_, SqlitePool>,
    settings: Settings,
) -> Result<Settings, AppError> {
    settings.validate()?;

    let mut tx = pool.begin().await?;
    for (key, value) in settings.to_pairs() {
        repository::upsert(&mut tx, key, &value).await?;
    }
    tx.commit().await?;

    Ok(settings)
}
