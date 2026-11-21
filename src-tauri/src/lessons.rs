// Module "lessons" : commandes Tauri liées aux leçons.
use sqlx::SqlitePool;

// Définition de la structure représentant une leçon, avec sérialisation JSON et mappage SQL.
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct Lesson {
    id: i64,
    title: String,
    date: String,
    // notes est optionnel, donc on utilise Option<String> pour représenter la possibilité d'une valeur nulle.
    notes: Option<String>,
    created_at: String,
}

// Structure pour la création d'une leçon, avec désérialisation JSON.
#[derive(serde::Deserialize)]
pub struct CreateLessonInput {
    title: String,
    date: String,
    notes: Option<String>,
}

#[tauri::command]
pub async fn list_lessons(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<Lesson>, String> {
    let lessons = sqlx::query_as::<_, Lesson>(
        r#"
        SELECT id, title, date, notes, created_at
        FROM lesson
        ORDER BY date DESC
        "#,
    )
    // fetch_all exécute la requête et récupère toutes les lignes correspondantes, en les mappant sur la structure Lesson.
    .fetch_all(pool.inner())
    .await
    // map_err convertit l'erreur SQLx en String pour la retourner à Tauri.
    .map_err(|e| e.to_string())?;

    // Si tout s'est bien passé, on retourne la liste des leçons.
    Ok(lessons)
}

#[tauri::command]
pub async fn create_lesson(
    pool: tauri::State<'_, SqlitePool>,
    input: CreateLessonInput,
) -> Result<Lesson, String> {
    if input.title.trim().is_empty() {
        return Err("Le titre de la leçon ne peut pas être vide".into());
    }

    if input.date.trim().is_empty() {
        return Err("La date de la leçon ne peut pas être vide".into());
    }

    // Insertion d'une nouvelle leçon dans la base de données.
    let created_lesson = sqlx::query_as::<_, Lesson>(
        r#"
        INSERT INTO lesson (title, date, notes)
        VALUES (?, ?, ?)
        RETURNING id, title, date, notes, created_at
        "#,
    )
    .bind(input.title)
    .bind(input.date)
    .bind(input.notes)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(created_lesson)
}

#[tauri::command]
pub async fn update_lesson(
    pool: tauri::State<'_, SqlitePool>,
    id: i64,
    input: CreateLessonInput,
) -> Result<Lesson, String> {
    if input.title.trim().is_empty() {
        return Err("Le titre de la leçon ne peut pas être vide".into());
    }

    if input.date.trim().is_empty() {
        return Err("La date de la leçon ne peut pas être vide".into());
    }

    let updated_lesson = sqlx::query_as::<_, Lesson>(
        r#"
        UPDATE lesson
        SET title = ?, date = ?, notes = ?
        WHERE id = ?
        RETURNING id, title, date, notes, created_at
        "#,
    )
    .bind(input.title)
    .bind(input.date)
    .bind(input.notes)
    .bind(id)
    .fetch_optional(pool.inner()) // Option<Lesson> : None si aucune ligne n'a été modifiée
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Leçon introuvable".to_string())?; // None devient Err("Leçon introuvable")

    Ok(updated_lesson)
}

#[tauri::command]
pub async fn delete_lesson(pool: tauri::State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    let result = sqlx::query(
        r#"
        DELETE FROM lesson
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Leçon introuvable".into());
    }

    Ok(())
}
