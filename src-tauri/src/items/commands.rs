// Commandes Tauri des éléments : valident, appellent le repository, convertissent les lignes en `Item`.

use sqlx::SqlitePool;
use tauri::State;

use super::model::{CreateItemInput, Item, ItemFilter, ItemRow};
use super::repository;
use crate::error::{AppError, is_foreign_key_violation};

// Une clé étrangère violée à l'écriture d'un élément = lesson_id inexistant.
fn lesson_not_found_or(error: sqlx::Error) -> AppError {
    if is_foreign_key_violation(&error) {
        AppError::NotFound("Leçon")
    } else {
        AppError::from(error)
    }
}

// JS : invoke("list_items") pour tout, ou invoke("list_items", { filter: { lesson_id: 3, kind: "vocab" } }).
#[tauri::command]
pub(crate) async fn list_items(
    pool: State<'_, SqlitePool>,
    filter: Option<ItemFilter>,
) -> Result<Vec<Item>, AppError> {
    let filter = filter.unwrap_or_default(); // absent => aucun filtre
    let rows = repository::find_all(pool.inner(), &filter).await?;

    // collect sur des Result : s'arrête à la première ligne invalide et renvoie son erreur.
    rows.into_iter().map(ItemRow::into_item).collect()
}

#[tauri::command]
pub(crate) async fn create_item(
    pool: State<'_, SqlitePool>,
    input: CreateItemInput,
) -> Result<Item, AppError> {
    input.validate()?;

    repository::insert(pool.inner(), &input)
        .await
        .map_err(lesson_not_found_or)?
        .into_item()
}

#[tauri::command]
pub(crate) async fn update_item(
    pool: State<'_, SqlitePool>,
    id: i64,
    input: CreateItemInput,
) -> Result<Item, AppError> {
    input.validate()?;

    repository::update(pool.inner(), id, &input)
        .await
        .map_err(lesson_not_found_or)?
        .ok_or(AppError::NotFound("Élément"))?
        .into_item()
}

#[tauri::command]
pub(crate) async fn set_item_active(
    pool: State<'_, SqlitePool>,
    id: i64,
    active: bool,
) -> Result<Item, AppError> {
    repository::set_active(pool.inner(), id, active)
        .await?
        .ok_or(AppError::NotFound("Élément"))?
        .into_item()
}

#[tauri::command]
pub(crate) async fn delete_item(pool: State<'_, SqlitePool>, id: i64) -> Result<(), AppError> {
    let deleted = repository::delete(pool.inner(), id).await?;

    if !deleted {
        return Err(AppError::NotFound("Élément"));
    }
    Ok(())
}
