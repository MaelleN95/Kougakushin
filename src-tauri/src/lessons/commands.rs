// Commandes Tauri des leçons (équivalent des contrôleurs) :
// valident l'entrée, appellent le repository, traduisent les erreurs pour le front.

use sqlx::SqlitePool;
use tauri::State;

use super::model::{CreateLessonInput, Lesson};
use super::repository;

// Refuse un titre ou une date vides (espaces seuls compris).
fn validate_input(input: &CreateLessonInput) -> Result<(), String> {
    if input.title.trim().is_empty() {
        return Err("Le titre de la leçon ne peut pas être vide".into());
    }
    if input.date.trim().is_empty() {
        return Err("La date de la leçon ne peut pas être vide".into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn list_lessons(pool: State<'_, SqlitePool>) -> Result<Vec<Lesson>, String> {
    repository::find_all(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn create_lesson(
    pool: State<'_, SqlitePool>,
    input: CreateLessonInput,
) -> Result<Lesson, String> {
    validate_input(&input)?;

    // as_deref : Option<String> -> Option<&str>, sans copier le texte.
    repository::insert(
        pool.inner(),
        &input.title,
        &input.date,
        input.notes.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn update_lesson(
    pool: State<'_, SqlitePool>,
    id: i64,
    input: CreateLessonInput,
) -> Result<Lesson, String> {
    validate_input(&input)?;

    repository::update(
        pool.inner(),
        id,
        &input.title,
        &input.date,
        input.notes.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())? // échec technique
    .ok_or_else(|| "Leçon introuvable".to_string()) // None => erreur métier (type absence de la leçon)
}

#[tauri::command]
pub(crate) async fn delete_lesson(pool: State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    let deleted = repository::delete(pool.inner(), id)
        .await
        .map_err(|e| e.to_string())?;

    if !deleted {
        return Err("Leçon introuvable".into());
    }
    Ok(())
}
