// Accès SQL aux éléments. Renvoie des lignes brutes (`ItemRow`) et les erreurs sqlx telles quelles.

use sqlx::SqlitePool;

use super::model::{CreateItemInput, ItemContent, ItemFilter, ItemRow};

// Valeurs des colonnes propres au type ; celles de l'autre type restent None (=> NULL,
// exigé par le CHECK de la migration). Partagé par `insert` et `update`.
// <'a> : la struct emprunte les textes de `ItemContent`, elle ne peut pas lui survivre.
struct ContentColumns<'a> {
    kind: &'static str,
    kana: Option<&'a str>,
    romaji: Option<&'a str>,
    kanji: Option<&'a str>,
    meaning: Option<&'a str>,
    word_type: Option<&'static str>,
    title: Option<&'a str>,
    explanation: Option<&'a str>,
}

fn content_columns(content: &ItemContent) -> ContentColumns<'_> {
    match content {
        ItemContent::Vocab {
            kana,
            romaji,
            kanji,
            meaning,
            word_type,
        } => ContentColumns {
            kind: "vocab",
            kana: Some(kana.as_str()),
            romaji: romaji.as_deref(),
            kanji: kanji.as_deref(),
            meaning: Some(meaning.as_str()),
            word_type: word_type.map(|w| w.as_str()),
            title: None,
            explanation: None,
        },
        ItemContent::Grammar { title, explanation } => ContentColumns {
            kind: "grammar",
            kana: None,
            romaji: None,
            kanji: None,
            meaning: None,
            word_type: None,
            title: Some(title.as_str()),
            explanation: Some(explanation.as_str()),
        },
    }
}

// Motif LIKE "contient `search`". Les caractères spéciaux de LIKE (% et _) sont échappés
// pour être cherchés tels quels. None si la recherche est vide.
fn like_pattern(search: Option<&str>) -> Option<String> {
    let search = search?.trim();
    if search.is_empty() {
        return None;
    }
    let escaped = search
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

// Éléments correspondant au filtre, les plus récents d'abord.
// Chaque critère vaut NULL quand il est absent, et `?n IS NULL` le neutralise.
pub(super) async fn find_all(
    pool: &SqlitePool,
    filter: &ItemFilter,
) -> Result<Vec<ItemRow>, sqlx::Error> {
    sqlx::query_as::<_, ItemRow>(
        r#"
        SELECT id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type,
               title, explanation, example_jp, example_fr, created_at, updated_at
        FROM item
        WHERE (?1 IS NULL OR lesson_id = ?1)
          AND (?2 IS NULL OR kind = ?2)
          AND (?3 IS NULL OR active = ?3)
          AND (?4 IS NULL
               OR kana LIKE ?4 ESCAPE '\' OR romaji LIKE ?4 ESCAPE '\'
               OR kanji LIKE ?4 ESCAPE '\' OR meaning LIKE ?4 ESCAPE '\'
               OR title LIKE ?4 ESCAPE '\' OR explanation LIKE ?4 ESCAPE '\')
        ORDER BY created_at DESC, id DESC
        "#,
    )
    .bind(filter.lesson_id)
    .bind(filter.kind.map(|k| k.as_str()))
    .bind(filter.active)
    .bind(like_pattern(filter.search.as_deref()))
    .fetch_all(pool)
    .await
}

// Insère un élément et renvoie la ligne enregistrée.
pub(super) async fn insert(
    pool: &SqlitePool,
    input: &CreateItemInput,
) -> Result<ItemRow, sqlx::Error> {
    let columns = content_columns(&input.content);

    sqlx::query_as::<_, ItemRow>(
        r#"
        INSERT INTO item (lesson_id, kind, kana, romaji, kanji, meaning, word_type,
                          title, explanation, example_jp, example_fr)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type,
                  title, explanation, example_jp, example_fr, created_at, updated_at
        "#,
    )
    .bind(input.lesson_id)
    .bind(columns.kind)
    .bind(columns.kana)
    .bind(columns.romaji)
    .bind(columns.kanji)
    .bind(columns.meaning)
    .bind(columns.word_type)
    .bind(columns.title)
    .bind(columns.explanation)
    .bind(&input.example_jp)
    .bind(&input.example_fr)
    .fetch_one(pool)
    .await
}

// Remplace tous les champs d'un élément (le type peut changer) ; None si l'id n'existe pas.
pub(super) async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &CreateItemInput,
) -> Result<Option<ItemRow>, sqlx::Error> {
    let columns = content_columns(&input.content);

    sqlx::query_as::<_, ItemRow>(
        r#"
        UPDATE item
        SET lesson_id = ?, kind = ?, kana = ?, romaji = ?, kanji = ?, meaning = ?,
            word_type = ?, title = ?, explanation = ?, example_jp = ?, example_fr = ?,
            updated_at = datetime('now')
        WHERE id = ?
        RETURNING id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type,
                  title, explanation, example_jp, example_fr, created_at, updated_at
        "#,
    )
    .bind(input.lesson_id)
    .bind(columns.kind)
    .bind(columns.kana)
    .bind(columns.romaji)
    .bind(columns.kanji)
    .bind(columns.meaning)
    .bind(columns.word_type)
    .bind(columns.title)
    .bind(columns.explanation)
    .bind(&input.example_jp)
    .bind(&input.example_fr)
    .bind(id)
    .fetch_optional(pool)
    .await
}

// Active ou désactive un élément ; None si l'id n'existe pas.
pub(super) async fn set_active(
    pool: &SqlitePool,
    id: i64,
    active: bool,
) -> Result<Option<ItemRow>, sqlx::Error> {
    sqlx::query_as::<_, ItemRow>(
        r#"
        UPDATE item
        SET active = ?, updated_at = datetime('now')
        WHERE id = ?
        RETURNING id, lesson_id, kind, active, kana, romaji, kanji, meaning, word_type,
                  title, explanation, example_jp, example_fr, created_at, updated_at
        "#,
    )
    .bind(active)
    .bind(id)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::super::model::{CreateItemInput, ItemContent, ItemFilter, ItemKind, ItemRow};
    use crate::error::is_foreign_key_violation;

    // Base en mémoire avec les vraies migrations et deux leçons (ids 1 et 2).
    // Une seule connexion : en mémoire, chaque connexion aurait sa propre base vide.
    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO lesson (title, date) VALUES ('L1', '2026-11-18'), ('L2', '2026-11-25')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn vocab(lesson_id: i64, kana: &str, meaning: &str) -> CreateItemInput {
        CreateItemInput {
            lesson_id,
            content: ItemContent::Vocab {
                kana: kana.into(),
                romaji: None,
                kanji: None,
                meaning: meaning.into(),
                word_type: None,
            },
            example_jp: None,
            example_fr: None,
        }
    }

    fn grammar(lesson_id: i64, title: &str) -> CreateItemInput {
        CreateItemInput {
            lesson_id,
            content: ItemContent::Grammar {
                title: title.into(),
                explanation: "explication".into(),
            },
            example_jp: None,
            example_fr: None,
        }
    }

    // Ligne => JSON tel que le front le reçoit.
    fn json(row: ItemRow) -> serde_json::Value {
        serde_json::to_value(row.into_item().unwrap()).unwrap()
    }

    // Ids renvoyés par find_all pour un filtre donné.
    async fn ids(pool: &SqlitePool, filter: ItemFilter) -> Vec<i64> {
        super::find_all(pool, &filter)
            .await
            .unwrap()
            .into_iter()
            .map(|row| json(row)["id"].as_i64().unwrap())
            .collect()
    }

    // Jeu de données : 1 = たべる (L1), 2 = grammaire (L1), 3 = のむ (L2).
    async fn seeded_pool() -> SqlitePool {
        let pool = test_pool().await;
        for input in [
            vocab(1, "たべる", "manger"),
            grammar(1, "〜ています"),
            vocab(2, "のむ", "boire"),
        ] {
            super::insert(&pool, &input).await.unwrap();
        }
        pool
    }

    #[test]
    fn sans_filtre_tout_est_renvoye_du_plus_recent_au_plus_ancien() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            assert_eq!(ids(&pool, ItemFilter::default()).await, vec![3, 2, 1]);
        });
    }

    #[test]
    fn filtre_par_lecon_et_par_type() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            let by_lesson = ItemFilter {
                lesson_id: Some(1),
                ..Default::default()
            };
            assert_eq!(ids(&pool, by_lesson).await, vec![2, 1]);
            let by_kind = ItemFilter {
                kind: Some(ItemKind::Vocab),
                ..Default::default()
            };
            assert_eq!(ids(&pool, by_kind).await, vec![3, 1]);
            let both = ItemFilter {
                lesson_id: Some(1),
                kind: Some(ItemKind::Grammar),
                ..Default::default()
            };
            assert_eq!(ids(&pool, both).await, vec![2]);
        });
    }

    #[test]
    fn filtre_par_actif_apres_desactivation() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            let row = super::set_active(&pool, 1, false).await.unwrap().unwrap();
            assert_eq!(json(row)["active"], false);
            let active = ItemFilter {
                active: Some(true),
                ..Default::default()
            };
            assert_eq!(ids(&pool, active).await, vec![3, 2]);
            let inactive = ItemFilter {
                active: Some(false),
                ..Default::default()
            };
            assert_eq!(ids(&pool, inactive).await, vec![1]);
        });
    }

    #[test]
    fn recherche_dans_les_champs_texte() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            let search = |s: &str| ItemFilter {
                search: Some(s.into()),
                ..Default::default()
            };
            assert_eq!(ids(&pool, search("たべ")).await, vec![1]); // kana
            assert_eq!(ids(&pool, search("MANGER")).await, vec![1]); // casse ASCII ignorée
            assert_eq!(ids(&pool, search("ています")).await, vec![2]); // titre de grammaire
            assert_eq!(ids(&pool, search("   ")).await, vec![3, 2, 1]); // vide => pas de filtre
        });
    }

    #[test]
    fn recherche_echappe_les_caracteres_speciaux_de_like() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            super::insert(&pool, &vocab(1, "a", "100%")).await.unwrap(); // id 1
            super::insert(&pool, &vocab(1, "b", "1000")).await.unwrap(); // id 2
            super::insert(&pool, &vocab(1, "c", "x_y")).await.unwrap(); // id 3
            super::insert(&pool, &vocab(1, "d", "xzy")).await.unwrap(); // id 4
            let search = |s: &str| ItemFilter {
                search: Some(s.into()),
                ..Default::default()
            };
            // Sans échappement, "0%" trouverait aussi "1000", et "x_y" trouverait "xzy".
            assert_eq!(ids(&pool, search("0%")).await, vec![1]);
            assert_eq!(ids(&pool, search("x_y")).await, vec![3]);
        });
    }

    #[test]
    fn modifier_peut_changer_le_type_et_vide_les_anciennes_colonnes() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            let row = super::update(&pool, 1, &grammar(2, "forme て"))
                .await
                .unwrap()
                .unwrap();
            let item = json(row);
            assert_eq!(item["lesson_id"], 2);
            assert_eq!(item["content"]["kind"], "grammar");
            assert_eq!(item["content"]["title"], "forme て");
            assert!(item["content"].get("kana").is_none());
        });
    }

    #[test]
    fn id_inconnu_donne_none() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            assert!(
                super::update(&pool, 999, &grammar(1, "x"))
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                super::set_active(&pool, 999, false)
                    .await
                    .unwrap()
                    .is_none()
            );
        });
    }

    #[test]
    fn lecon_inconnue_viole_la_cle_etrangere() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            let insert_error = super::insert(&pool, &grammar(999, "x")).await.unwrap_err();
            assert!(is_foreign_key_violation(&insert_error));
            let update_error = super::update(&pool, 1, &grammar(999, "x"))
                .await
                .unwrap_err();
            assert!(is_foreign_key_violation(&update_error));
        });
    }
}
