// Commandes Tauri des manuels : valident, appellent le repository, traduisent les erreurs.

use sqlx::SqlitePool;
use tauri::State;

use super::model::{Book, BookInput};
use super::repository;
use crate::error::{AppError, is_foreign_key_violation, is_unique_violation};

// Titre déjà pris (contrainte UNIQUE, casse ignorée) => message clair.
fn duplicate_or(error: sqlx::Error) -> AppError {
    if is_unique_violation(&error) {
        AppError::Conflict("Un manuel porte déjà ce titre.")
    } else {
        AppError::from(error)
    }
}

#[tauri::command]
pub(crate) async fn list_books(pool: State<'_, SqlitePool>) -> Result<Vec<Book>, AppError> {
    Ok(repository::find_all(pool.inner()).await?)
}

#[tauri::command]
pub(crate) async fn create_book(
    pool: State<'_, SqlitePool>,
    input: BookInput,
) -> Result<Book, AppError> {
    let title = input.validated_title()?;
    repository::insert(pool.inner(), title)
        .await
        .map_err(duplicate_or)
}

#[tauri::command]
pub(crate) async fn update_book(
    pool: State<'_, SqlitePool>,
    id: i64,
    input: BookInput,
) -> Result<Book, AppError> {
    let title = input.validated_title()?;
    repository::update(pool.inner(), id, title)
        .await
        .map_err(duplicate_or)?
        .ok_or(AppError::NotFound("Manuel"))
}

#[tauri::command]
pub(crate) async fn delete_book(pool: State<'_, SqlitePool>, id: i64) -> Result<(), AppError> {
    let deleted = repository::delete(pool.inner(), id).await.map_err(|e| {
        if is_foreign_key_violation(&e) {
            AppError::Conflict("Impossible de supprimer un manuel qui contient des leçons")
        } else {
            AppError::from(e)
        }
    })?;
    if !deleted {
        return Err(AppError::NotFound("Manuel"));
    }
    Ok(())
}
