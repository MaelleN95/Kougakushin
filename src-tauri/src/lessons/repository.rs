// Accès SQL aux leçons (équivalent d'un repository Doctrine).
// Ne connaît ni Tauri ni les messages métier : renvoie les erreurs sqlx telles quelles.

use sqlx::SqlitePool;

use super::model::Lesson;

// Toutes les leçons, la plus récente d'abord.
pub(super) async fn find_all(pool: &SqlitePool) -> Result<Vec<Lesson>, sqlx::Error> {
    sqlx::query_as::<_, Lesson>(
        r#"
        SELECT id, title, date, notes, created_at
        FROM lesson
        ORDER BY date DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

// Insère une leçon et la renvoie telle qu'enregistrée (id et created_at compris).
pub(super) async fn insert(
    pool: &SqlitePool,
    title: &str,
    date: &str,
    notes: Option<&str>,
) -> Result<Lesson, sqlx::Error> {
    sqlx::query_as::<_, Lesson>(
        r#"
        INSERT INTO lesson (title, date, notes)
        VALUES (?, ?, ?)
        RETURNING id, title, date, notes, created_at
        "#,
    )
    .bind(title)
    .bind(date)
    .bind(notes)
    .fetch_one(pool)
    .await
}

// Modifie une leçon ; None si l'id n'existe pas.
pub(super) async fn update(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    date: &str,
    notes: Option<&str>,
) -> Result<Option<Lesson>, sqlx::Error> {
    sqlx::query_as::<_, Lesson>(
        r#"
        UPDATE lesson
        SET title = ?, date = ?, notes = ?
        WHERE id = ?
        RETURNING id, title, date, notes, created_at
        "#,
    )
    .bind(title)
    .bind(date)
    .bind(notes)
    .bind(id)
    .fetch_optional(pool)
    .await
}

// Supprime une leçon ; false si l'id n'existe pas.
pub(super) async fn delete(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM lesson WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
