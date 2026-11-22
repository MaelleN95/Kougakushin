// Accès SQL aux éléments. Renvoie des lignes brutes (`ItemRow`) et les erreurs sqlx telles quelles.

use sqlx::SqlitePool;

use super::model::{CreateItemInput, ItemContent, ItemRow};

// Tous les éléments, ou ceux d'une seule leçon ; les plus récents d'abord.
pub(super) async fn find_all(
    pool: &SqlitePool,
    lesson_id: Option<i64>,
) -> Result<Vec<ItemRow>, sqlx::Error> {
    // ?1 utilisé deux fois : NULL => pas de filtre.
    sqlx::query_as::<_, ItemRow>(
        r#"
        SELECT id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type,
               title, explanation, example_jp, example_fr, created_at, updated_at
        FROM item
        WHERE (?1 IS NULL OR lesson_id = ?1)
        ORDER BY created_at DESC, id DESC
        "#,
    )
    .bind(lesson_id)
    .fetch_all(pool)
    .await
}

// Insère un élément et renvoie la ligne enregistrée.
pub(super) async fn insert(
    pool: &SqlitePool,
    input: &CreateItemInput,
) -> Result<ItemRow, sqlx::Error> {
    let query = sqlx::query_as::<_, ItemRow>(
        r#"
        INSERT INTO item (lesson_id, kind, kana, romaji, kanji, meaning, word_type,
                          title, explanation, example_jp, example_fr)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type,
                  title, explanation, example_jp, example_fr, created_at, updated_at
        "#,
    )
    .bind(input.lesson_id);

    // Les colonnes de l'autre type restent NULL (exigé par le CHECK de la migration).
    let query = match &input.content {
        ItemContent::Vocab {
            kana,
            romaji,
            kanji,
            meaning,
            word_type,
        } => query
            .bind("vocab")
            .bind(kana)
            .bind(romaji)
            .bind(kanji)
            .bind(meaning)
            .bind(word_type.map(|w| w.as_str()))
            .bind(None::<&str>) // title
            .bind(None::<&str>), // explanation
        ItemContent::Grammar { title, explanation } => query
            .bind("grammar")
            .bind(None::<&str>) // kana
            .bind(None::<&str>) // romaji
            .bind(None::<&str>) // kanji
            .bind(None::<&str>) // meaning
            .bind(None::<&str>) // word_type
            .bind(title)
            .bind(explanation),
    };

    query
        .bind(&input.example_jp)
        .bind(&input.example_fr)
        .fetch_one(pool)
        .await
}
