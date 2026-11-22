// Commandes Tauri des éléments : valident, appellent le repository, convertissent les lignes en `Item`.

use sqlx::SqlitePool;
use tauri::State;

use super::model::{CreateItemInput, Item, ItemRow};
use super::repository;
use crate::error::{AppError, is_foreign_key_violation};

// JS : invoke("list_items", { lessonId: 3 }) ou invoke("list_items") pour tout.
// Tauri convertit les noms d'arguments en camelCase côté JS.
#[tauri::command]
pub(crate) async fn list_items(
    pool: State<'_, SqlitePool>,
    lesson_id: Option<i64>,
) -> Result<Vec<Item>, AppError> {
    let rows = repository::find_all(pool.inner(), lesson_id).await?;

    // collect sur des Result : s'arrête à la première ligne invalide et renvoie son erreur.
    rows.into_iter().map(ItemRow::into_item).collect()
}

#[tauri::command]
pub(crate) async fn create_item(
    pool: State<'_, SqlitePool>,
    input: CreateItemInput,
) -> Result<Item, AppError> {
    input.validate()?;

    let row = repository::insert(pool.inner(), &input)
        .await
        .map_err(|e| match e {
            // lesson_id ne correspond à aucune leçon.
            e if is_foreign_key_violation(&e) => AppError::NotFound("Leçon"),
            e => AppError::from(e),
        })?;

    row.into_item()
}
