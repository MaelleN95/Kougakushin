// Accès SQL de la sauvegarde : lecture de toutes les tables, remplacement complet.

use sqlx::SqlitePool;

use super::model::{
    Backup, BookRecord, CardRecord, ConjugationFormRecord, FORMAT, ItemRecord, LessonRecord,
    ReviewLogRecord, SettingRecord, VERSION,
};

// Toutes les données, dans l'ordre des id.
pub(super) async fn snapshot(
    pool: &SqlitePool,
    exported_at: String,
) -> Result<Backup, sqlx::Error> {
    Ok(Backup {
        format: FORMAT.to_string(),
        version: VERSION,
        exported_at,
        books: sqlx::query_as::<_, BookRecord>(
            "SELECT id, title, created_at FROM book ORDER BY id",
        )
        .fetch_all(pool)
        .await?,
        lessons: sqlx::query_as::<_, LessonRecord>(
            "SELECT id, book_id, title, date, notes, created_at FROM lesson ORDER BY id",
        )
        .fetch_all(pool)
        .await?,
        items: sqlx::query_as::<_, ItemRecord>(
            "SELECT id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type, title,
                    explanation, example_jp, example_fr, created_at, updated_at
             FROM item ORDER BY id",
        )
        .fetch_all(pool)
        .await?,
        cards: sqlx::query_as::<_, CardRecord>(
            "SELECT id, item_id, direction, form, next_review_at, stability, difficulty,
                    elapsed_days, scheduled_days, reps, lapses, state, last_review, created_at
             FROM card ORDER BY id",
        )
        .fetch_all(pool)
        .await?,
        review_logs: sqlx::query_as::<_, ReviewLogRecord>(
            "SELECT id, card_id, rating, state, elapsed_days, scheduled_days, reviewed_at
             FROM review_log ORDER BY id",
        )
        .fetch_all(pool)
        .await?,
        settings: sqlx::query_as::<_, SettingRecord>("SELECT key, value FROM setting ORDER BY key")
            .fetch_all(pool)
            .await?,
        conjugation_forms: sqlx::query_as::<_, ConjugationFormRecord>(
            "SELECT form, known FROM conjugation_form ORDER BY form",
        )
        .fetch_all(pool)
        .await?,
    })
}

// Remplace TOUTES les données par celles de la sauvegarde, en une transaction :
// si une ligne viole une contrainte (CHECK, clé étrangère...), rien n'est modifié.
// Les id sont conservés, pour que les liens manuel / leçon / élément / carte / historique restent justes.
pub(super) async fn replace_all(pool: &SqlitePool, backup: &Backup) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Enfants d'abord, à cause des clés étrangères.
    // Requêtes écrites en entier (pas de nom de table assemblé) : sqlx n'accepte que du SQL fixe.
    for delete in [
        "DELETE FROM review_log",
        "DELETE FROM card",
        "DELETE FROM item",
        "DELETE FROM lesson",
        "DELETE FROM book",
        "DELETE FROM setting",
        "DELETE FROM conjugation_form",
    ] {
        sqlx::query(delete).execute(&mut *tx).await?;
    }

    for b in &backup.books {
        sqlx::query("INSERT INTO book (id, title, created_at) VALUES (?, ?, ?)")
            .bind(b.id)
            .bind(&b.title)
            .bind(&b.created_at)
            .execute(&mut *tx)
            .await?;
    }
    for l in &backup.lessons {
        sqlx::query(
            "INSERT INTO lesson (id, book_id, title, date, notes, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(l.id)
        .bind(l.book_id)
        .bind(&l.title)
        .bind(&l.date)
        .bind(&l.notes)
        .bind(&l.created_at)
        .execute(&mut *tx)
        .await?;
    }
    for i in &backup.items {
        sqlx::query(
            "INSERT INTO item (id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type,
                               title, explanation, example_jp, example_fr, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(i.id)
        .bind(i.lesson_id)
        .bind(&i.kind)
        .bind(i.active)
        .bind(&i.kana)
        .bind(&i.romaji)
        .bind(&i.kanji)
        .bind(&i.meaning)
        .bind(&i.word_type)
        .bind(&i.title)
        .bind(&i.explanation)
        .bind(&i.example_jp)
        .bind(&i.example_fr)
        .bind(&i.created_at)
        .bind(&i.updated_at)
        .execute(&mut *tx)
        .await?;
    }
    for c in &backup.cards {
        sqlx::query(
            "INSERT INTO card (id, item_id, direction, form, next_review_at, stability, difficulty,
                               elapsed_days, scheduled_days, reps, lapses, state, last_review,
                               created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(c.id)
        .bind(c.item_id)
        .bind(&c.direction)
        .bind(&c.form)
        .bind(&c.next_review_at)
        .bind(c.stability)
        .bind(c.difficulty)
        .bind(c.elapsed_days)
        .bind(c.scheduled_days)
        .bind(c.reps)
        .bind(c.lapses)
        .bind(c.state)
        .bind(&c.last_review)
        .bind(&c.created_at)
        .execute(&mut *tx)
        .await?;
    }
    for r in &backup.review_logs {
        sqlx::query(
            "INSERT INTO review_log (id, card_id, rating, state, elapsed_days, scheduled_days,
                                     reviewed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(r.id)
        .bind(r.card_id)
        .bind(r.rating)
        .bind(r.state)
        .bind(r.elapsed_days)
        .bind(r.scheduled_days)
        .bind(&r.reviewed_at)
        .execute(&mut *tx)
        .await?;
    }
    for s in &backup.settings {
        sqlx::query("INSERT INTO setting (key, value) VALUES (?, ?)")
            .bind(&s.key)
            .bind(&s.value)
            .execute(&mut *tx)
            .await?;
    }
    for f in &backup.conjugation_forms {
        sqlx::query("INSERT INTO conjugation_form (form, known) VALUES (?, ?)")
            .bind(&f.form)
            .bind(f.known)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await
}
