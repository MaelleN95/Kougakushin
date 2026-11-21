// Module « lessons » : commandes Tauri liées aux leçons.
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
