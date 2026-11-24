// Commandes Tauri des leçons (équivalent des contrôleurs) :
// valident l'entrée, appellent le repository, traduisent les absences en erreurs.

use sqlx::SqlitePool;
use tauri::State;

use super::model::{CreateLessonInput, Lesson};
use super::repository;
use crate::error::{AppError, is_foreign_key_violation};

// Le `?` convertit les sqlx::Error en AppError grâce au #[from].

// Une clé étrangère violée à l'écriture d'une leçon = book_id inexistant.
fn book_not_found_or(error: sqlx::Error) -> AppError {
    if is_foreign_key_violation(&error) {
        AppError::NotFound("Manuel")
    } else {
        AppError::from(error)
    }
}

#[tauri::command]
pub(crate) async fn list_lessons(pool: State<'_, SqlitePool>) -> Result<Vec<Lesson>, AppError> {
    Ok(repository::find_all(pool.inner()).await?)
}

#[tauri::command]
pub(crate) async fn create_lesson(
    pool: State<'_, SqlitePool>,
    input: CreateLessonInput,
) -> Result<Lesson, AppError> {
    input.validate()?;

    // as_deref : Option<String> -> Option<&str>, sans copier le texte.
    let lesson = repository::insert(
        pool.inner(),
        input.book_id,
        &input.title,
        &input.date,
        input.notes.as_deref(),
    )
    .await
    .map_err(book_not_found_or)?;

    Ok(lesson)
}

#[tauri::command]
pub(crate) async fn update_lesson(
    pool: State<'_, SqlitePool>,
    id: i64,
    input: CreateLessonInput,
) -> Result<Lesson, AppError> {
    input.validate()?;

    repository::update(
        pool.inner(),
        id,
        input.book_id,
        &input.title,
        &input.date,
        input.notes.as_deref(),
    )
    .await
    .map_err(book_not_found_or)? // manuel absent ou échec technique
    .ok_or(AppError::NotFound("Leçon")) // None => leçon absente
}

#[tauri::command]
pub(crate) async fn delete_lesson(pool: State<'_, SqlitePool>, id: i64) -> Result<(), AppError> {
    let deleted = repository::delete(pool.inner(), id)
        .await
        .map_err(|e| match e {
            // Clé étrangère de la table item : la leçon contient encore des éléments.
            e if is_foreign_key_violation(&e) => {
                AppError::Conflict("Impossible de supprimer une leçon qui contient des éléments")
            }
            e => AppError::from(e),
        })?;

    if !deleted {
        return Err(AppError::NotFound("Leçon"));
    }
    Ok(())
}
