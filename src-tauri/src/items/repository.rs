// Accès SQL aux éléments. Renvoie des lignes brutes (`ItemRow`) et les erreurs sqlx telles quelles.

use sqlx::{Executor, Sqlite, SqliteConnection, SqlitePool};

use super::model::{CreateItemInput, ItemContent, ItemFilter, ItemRow, clean_meanings};

// Début commun des lectures d'un ItemRow. `meanings` : sens de la table item_meaning, en
// tableau JSON dans l'ordre (`[]` pour la grammaire). La colonne item.meaning n'est pas lue :
// c'est un texte dérivé (voir la migration 20261126000100).
// Macro plutôt que constante : concat! n'accepte que des littéraux.
macro_rules! select_item {
    () => {
        r#"
        SELECT id, lesson_id, kind, active, kana, romaji, kanji, word_type,
               title, explanation, example_jp, example_fr, created_at, updated_at,
               (SELECT json_group_array(m.meaning ORDER BY m.position)
                FROM item_meaning m WHERE m.item_id = item.id) AS meanings
        FROM item
        "#
    };
}

// Valeurs des colonnes propres au type ; celles de l'autre type restent None (=> NULL,
// exigé par le CHECK de la migration). Partagé par `insert` et `update`.
// <'a> : la struct emprunte les textes de `ItemContent`, elle ne peut pas lui survivre.
struct ContentColumns<'a> {
    kind: &'static str,
    kana: Option<&'a str>,
    romaji: Option<&'a str>,
    kanji: Option<&'a str>,
    meaning: Option<String>, // texte dérivé des sens ("je; moi"), exigé par le CHECK
    meanings: Vec<String>,   // sens nettoyés, écrits dans item_meaning
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
            meanings,
            word_type,
        } => {
            let meanings = clean_meanings(meanings);
            ContentColumns {
                kind: "vocab",
                kana: Some(kana.as_str()),
                romaji: romaji.as_deref(),
                kanji: kanji.as_deref(),
                meaning: Some(meanings.join("; ")),
                meanings,
                word_type: word_type.map(|w| w.as_str()),
                title: None,
                explanation: None,
            }
        }
        ItemContent::Grammar { title, explanation } => ContentColumns {
            kind: "grammar",
            kana: None,
            romaji: None,
            kanji: None,
            meaning: None,
            meanings: Vec::new(),
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
    sqlx::query_as::<_, ItemRow>(concat!(
        select_item!(),
        r#"
        WHERE (?1 IS NULL OR lesson_id = ?1)
          AND (?2 IS NULL OR kind = ?2)
          AND (?3 IS NULL OR active = ?3)
          AND (?4 IS NULL
               OR kana LIKE ?4 ESCAPE '\' OR romaji LIKE ?4 ESCAPE '\'
               OR kanji LIKE ?4 ESCAPE '\'
               OR EXISTS (SELECT 1 FROM item_meaning m
                          WHERE m.item_id = item.id AND m.meaning LIKE ?4 ESCAPE '\')
               OR title LIKE ?4 ESCAPE '\' OR explanation LIKE ?4 ESCAPE '\')
        ORDER BY created_at DESC, id DESC
        "#
    ))
    .bind(filter.lesson_id)
    .bind(filter.kind.map(|k| k.as_str()))
    .bind(filter.active)
    .bind(like_pattern(filter.search.as_deref()))
    .fetch_all(pool)
    .await
}

// Un élément et ses sens ; None si l'id n'existe pas.
async fn find_by_id<'c, E>(executor: E, id: i64) -> Result<Option<ItemRow>, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    sqlx::query_as::<_, ItemRow>(concat!(select_item!(), "WHERE id = ?"))
        .bind(id)
        .fetch_optional(executor)
        .await
}

// Remplace les sens d'un élément par `meanings` (aucun pour la grammaire).
async fn replace_meanings(
    conn: &mut SqliteConnection,
    item_id: i64,
    meanings: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM item_meaning WHERE item_id = ?")
        .bind(item_id)
        .execute(&mut *conn)
        .await?;
    // (0..).zip : numérote les sens 0, 1, 2... directement en i64.
    for (position, meaning) in (0_i64..).zip(meanings) {
        sqlx::query("INSERT INTO item_meaning (item_id, position, meaning) VALUES (?, ?, ?)")
            .bind(item_id)
            .bind(position)
            .bind(meaning)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

// Insère un élément et ses sens, puis renvoie la ligne enregistrée.
// Plusieurs requêtes : à appeler dans une transaction (voir commands.rs).
pub(super) async fn insert(
    conn: &mut SqliteConnection,
    input: &CreateItemInput,
) -> Result<ItemRow, sqlx::Error> {
    let columns = content_columns(&input.content);

    let id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO item (lesson_id, kind, kana, romaji, kanji, meaning, word_type,
                          title, explanation, example_jp, example_fr)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(input.lesson_id)
    .bind(columns.kind)
    .bind(columns.kana)
    .bind(columns.romaji)
    .bind(columns.kanji)
    .bind(&columns.meaning)
    .bind(columns.word_type)
    .bind(columns.title)
    .bind(columns.explanation)
    .bind(&input.example_jp)
    .bind(&input.example_fr)
    .fetch_one(&mut *conn)
    .await?;

    replace_meanings(conn, id, &columns.meanings).await?;
    // La ligne vient d'être insérée sur la même connexion : elle existe forcément.
    find_by_id(&mut *conn, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

// Remplace tous les champs d'un élément et ses sens (le type peut changer) ; None si l'id
// n'existe pas. Plusieurs requêtes : à appeler dans une transaction.
pub(super) async fn update(
    conn: &mut SqliteConnection,
    id: i64,
    input: &CreateItemInput,
) -> Result<Option<ItemRow>, sqlx::Error> {
    let columns = content_columns(&input.content);

    let updated: Option<i64> = sqlx::query_scalar(
        r#"
        UPDATE item
        SET lesson_id = ?, kind = ?, kana = ?, romaji = ?, kanji = ?, meaning = ?,
            word_type = ?, title = ?, explanation = ?, example_jp = ?, example_fr = ?,
            updated_at = datetime('now')
        WHERE id = ?
        RETURNING id
        "#,
    )
    .bind(input.lesson_id)
    .bind(columns.kind)
    .bind(columns.kana)
    .bind(columns.romaji)
    .bind(columns.kanji)
    .bind(&columns.meaning)
    .bind(columns.word_type)
    .bind(columns.title)
    .bind(columns.explanation)
    .bind(&input.example_jp)
    .bind(&input.example_fr)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    if updated.is_none() {
        return Ok(None);
    }

    // Devenu grammaire : la liste vide efface les anciens sens.
    replace_meanings(conn, id, &columns.meanings).await?;
    find_by_id(&mut *conn, id).await
}

// Active ou désactive un élément ; None si l'id n'existe pas.
pub(super) async fn set_active(
    pool: &SqlitePool,
    id: i64,
    active: bool,
) -> Result<Option<ItemRow>, sqlx::Error> {
    let result =
        sqlx::query("UPDATE item SET active = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(active)
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    find_by_id(pool, id).await
}

// Vrai si un autre mot identique existe déjà : mêmes kana, mêmes kanji (ou aucun des deux) et même
// traduction (décision D21). Depuis D43, "même traduction" = les mêmes sens, dans n'importe quel
// ordre. Les homonymes aux kanji différents (橋 / 箸) restent permis.
// `exclude_id` : l'élément en cours de modification, qui ne compte pas comme doublon de lui-même.
pub(super) async fn vocab_duplicate_exists<'c, E>(
    executor: E,
    content: &ItemContent,
    exclude_id: Option<i64>,
) -> Result<bool, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let ItemContent::Vocab {
        kana,
        kanji,
        meanings,
        ..
    } = content
    else {
        return Ok(false); // la grammaire n'est pas concernée
    };
    // Sens de chaque mot de mêmes kana et kanji. Json<T> : décode le tableau JSON renvoyé.
    let candidates: Vec<sqlx::types::Json<Vec<String>>> = sqlx::query_scalar(
        r#"
        SELECT (SELECT json_group_array(m.meaning) FROM item_meaning m WHERE m.item_id = item.id)
        FROM item
        WHERE kind = 'vocab'
          AND trim(kana) = ?1
          AND COALESCE(trim(kanji), '') = ?2
          AND (?3 IS NULL OR id <> ?3)
        "#,
    )
    .bind(kana.trim())
    .bind(kanji.as_deref().map(str::trim).unwrap_or(""))
    .bind(exclude_id)
    .fetch_all(executor)
    .await?;

    // Comparaison sans tenir compte de l'ordre : on trie les deux listes.
    let mut wanted = clean_meanings(meanings);
    wanted.sort();
    Ok(candidates.into_iter().any(|candidate| {
        let mut existing = candidate.0;
        existing.sort();
        existing == wanted
    }))
}

// Supprime un élément ; false si l'id n'existe pas.
pub(super) async fn delete(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM item WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// --- Actions groupées (sélection de plusieurs éléments) ---
// Une seule requête par action : tout ou rien, sans transaction explicite.
// Les ids passent en un tableau JSON, déplié par json_each (pas de SQL assemblé à la main).

// [1,2,3] : des entiers seulement, donc aucune injection possible.
fn ids_json(ids: &[i64]) -> String {
    let list: Vec<String> = ids.iter().map(i64::to_string).collect();
    format!("[{}]", list.join(","))
}

// Nombre d'éléments modifiés (les ids inexistants sont ignorés).
pub(super) async fn set_active_many(
    pool: &SqlitePool,
    ids: &[i64],
    active: bool,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE item SET active = ?1, updated_at = datetime('now')
         WHERE id IN (SELECT value FROM json_each(?2))",
    )
    .bind(active)
    .bind(ids_json(ids))
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// Change la leçon ; les cartes et leur progression ne bougent pas (elles suivent l'élément).
pub(super) async fn move_many(
    pool: &SqlitePool,
    ids: &[i64],
    lesson_id: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE item SET lesson_id = ?1, updated_at = datetime('now')
         WHERE id IN (SELECT value FROM json_each(?2))",
    )
    .bind(lesson_id)
    .bind(ids_json(ids))
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// Supprime les éléments, leurs cartes et leur historique (CASCADE).
pub(super) async fn delete_many(pool: &SqlitePool, ids: &[i64]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM item WHERE id IN (SELECT value FROM json_each(?1))")
        .bind(ids_json(ids))
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
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
                meanings: vec![meaning.into()],
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
            super::insert(&mut pool.acquire().await.unwrap(), &input)
                .await
                .unwrap();
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
            super::insert(&mut pool.acquire().await.unwrap(), &vocab(1, "a", "100%"))
                .await
                .unwrap(); // id 1
            super::insert(&mut pool.acquire().await.unwrap(), &vocab(1, "b", "1000"))
                .await
                .unwrap(); // id 2
            super::insert(&mut pool.acquire().await.unwrap(), &vocab(1, "c", "x_y"))
                .await
                .unwrap(); // id 3
            super::insert(&mut pool.acquire().await.unwrap(), &vocab(1, "d", "xzy"))
                .await
                .unwrap(); // id 4
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
            let row = super::update(
                &mut pool.acquire().await.unwrap(),
                1,
                &grammar(2, "forme て"),
            )
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
    fn supprimer_renvoie_vrai_puis_faux() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            assert!(super::delete(&pool, 1).await.unwrap());
            assert!(!super::delete(&pool, 1).await.unwrap()); // déjà supprimé
            assert_eq!(ids(&pool, ItemFilter::default()).await, vec![3, 2]);
        });
    }

    #[test]
    fn id_inconnu_donne_none() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            assert!(
                super::update(&mut pool.acquire().await.unwrap(), 999, &grammar(1, "x"))
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
            let insert_error =
                super::insert(&mut pool.acquire().await.unwrap(), &grammar(999, "x"))
                    .await
                    .unwrap_err();
            assert!(is_foreign_key_violation(&insert_error));
            let update_error =
                super::update(&mut pool.acquire().await.unwrap(), 1, &grammar(999, "x"))
                    .await
                    .unwrap_err();
            assert!(is_foreign_key_violation(&update_error));
        });
    }

    #[test]
    fn doublon_exact_refuse_homonyme_permis() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await; // 1 = たべる (manger), sans kanji
            let same = vocab(1, " たべる ", "manger");
            assert!(
                super::vocab_duplicate_exists(&pool, &same.content, None)
                    .await
                    .unwrap()
            );
            // Le mot lui-même, en cours de modification, n'est pas un doublon.
            assert!(
                !super::vocab_duplicate_exists(&pool, &same.content, Some(1))
                    .await
                    .unwrap()
            );
            // Même kana, autre traduction : homonyme permis.
            let homonym = vocab(1, "たべる", "dévorer");
            assert!(
                !super::vocab_duplicate_exists(&pool, &homonym.content, None)
                    .await
                    .unwrap()
            );
            // La grammaire n'est pas concernée.
            assert!(
                !super::vocab_duplicate_exists(&pool, &grammar(1, "〜ています").content, None)
                    .await
                    .unwrap()
            );
        });
    }

    // Sens tels que renvoyés au front pour l'élément `id`.
    async fn meanings_of(pool: &SqlitePool, id: i64) -> serde_json::Value {
        let row = super::find_by_id(pool, id).await.unwrap().unwrap();
        json(row)["content"]["meanings"].clone()
    }

    #[test]
    fn plusieurs_sens_enregistres_dans_l_ordre_et_nettoyes() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let mut input = vocab(1, "わたし", "je");
            if let ItemContent::Vocab { meanings, .. } = &mut input.content {
                *meanings = vec![" je ".into(), "".into(), "moi".into(), "je".into()];
            }
            let row = super::insert(&mut pool.acquire().await.unwrap(), &input)
                .await
                .unwrap();
            assert_eq!(
                json(row)["content"]["meanings"],
                serde_json::json!(["je", "moi"])
            );
            // Texte dérivé, gardé pour le CHECK de la table item.
            let text: String = sqlx::query_scalar("SELECT meaning FROM item WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(text, "je; moi");
        });
    }

    #[test]
    fn modifier_remplace_les_sens_et_la_grammaire_n_en_a_aucun() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await; // 1 = たべる (manger)
            super::update(
                &mut pool.acquire().await.unwrap(),
                1,
                &vocab(1, "たべる", "manger; dévorer"),
            )
            .await
            .unwrap();
            assert_eq!(
                meanings_of(&pool, 1).await,
                serde_json::json!(["manger", "dévorer"])
            );

            super::update(&mut pool.acquire().await.unwrap(), 1, &grammar(1, "t"))
                .await
                .unwrap();
            let left: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM item_meaning WHERE item_id = 1")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(left, 0);
        });
    }

    #[test]
    fn recherche_dans_chaque_sens_et_suppression_en_cascade() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            super::insert(
                &mut pool.acquire().await.unwrap(),
                &vocab(1, "わたし", "je; moi"),
            )
            .await
            .unwrap();
            let search = ItemFilter {
                search: Some("moi".into()),
                ..Default::default()
            };
            assert_eq!(ids(&pool, search).await, vec![1]); // deuxième sens trouvé

            assert!(super::delete(&pool, 1).await.unwrap());
            let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_meaning")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(left, 0);
        });
    }

    #[test]
    fn doublon_compare_les_sens_sans_tenir_compte_de_l_ordre() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            super::insert(
                &mut pool.acquire().await.unwrap(),
                &vocab(1, "わたし", "je; moi"),
            )
            .await
            .unwrap();
            let exists = |meaning: &'static str| {
                let pool = pool.clone();
                async move {
                    super::vocab_duplicate_exists(&pool, &vocab(1, "わたし", meaning).content, None)
                        .await
                        .unwrap()
                }
            };
            assert!(exists("moi; je").await);
            assert!(!exists("je").await); // un seul des deux sens : pas un doublon
            assert!(!exists("je; moi; nous").await);
        });
    }

    #[test]
    fn la_migration_decoupe_les_anciennes_traductions() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            // Base dans l'état d'avant D43 : toutes les migrations sauf celle des sens.
            let migrator = sqlx::migrate!("./migrations");
            for migration in migrator.iter().filter(|m| m.version < 20261126000100) {
                sqlx::raw_sql(migration.sql.clone())
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            sqlx::raw_sql(
                "INSERT INTO lesson (title, date) VALUES ('L1', '2026-11-18');
                 INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (1, 'vocab', 'わたし', 'je; moi');
                 INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (1, 'vocab', 'た', ' a ;; b;a; ');
                 INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (1, 'vocab', 'み', 'voir');
                 INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (1, 'vocab', 'の', ' ; ');
                 INSERT INTO item (lesson_id, kind, title, explanation) VALUES (1, 'grammar', 't', 'e');",
            )
            .execute(&pool)
            .await
            .unwrap();

            sqlx::raw_sql(include_str!(
                "../../migrations/20261126000100_item_meanings.sql"
            ))
            .execute(&pool)
            .await
            .unwrap();

            assert_eq!(
                meanings_of(&pool, 1).await,
                serde_json::json!(["je", "moi"])
            );
            assert_eq!(meanings_of(&pool, 2).await, serde_json::json!(["a", "b"]));
            assert_eq!(meanings_of(&pool, 3).await, serde_json::json!(["voir"]));
            // Que des séparateurs : le texte d'origine est gardé plutôt que de perdre le mot.
            assert_eq!(meanings_of(&pool, 4).await, serde_json::json!([" ; "]));
            // Grammaire : aucun sens, et pas de champ `meanings` côté front.
            assert!(meanings_of(&pool, 5).await.is_null());
            let text: String = sqlx::query_scalar("SELECT meaning FROM item WHERE id = 2")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(text, "a; b");
        });
    }

    #[test]
    fn actions_groupees_activer_deplacer_supprimer() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            let inactive = || ItemFilter {
                active: Some(false),
                ..ItemFilter::default()
            };
            let in_lesson_2 = || ItemFilter {
                lesson_id: Some(2),
                ..ItemFilter::default()
            };

            // L'id 999 n'existe pas : ignoré, seuls 2 éléments sont comptés.
            assert_eq!(
                super::set_active_many(&pool, &[1, 2, 999], false)
                    .await
                    .unwrap(),
                2
            );
            assert_eq!(ids(&pool, inactive()).await, vec![2, 1]);

            assert_eq!(super::move_many(&pool, &[1, 2], 2).await.unwrap(), 2);
            assert_eq!(ids(&pool, in_lesson_2()).await, vec![3, 2, 1]);
            // Leçon inconnue : rien ne bouge.
            let error = super::move_many(&pool, &[1], 999).await.unwrap_err();
            assert!(is_foreign_key_violation(&error));
            assert_eq!(ids(&pool, in_lesson_2()).await, vec![3, 2, 1]);

            assert_eq!(super::delete_many(&pool, &[1, 3]).await.unwrap(), 2);
            assert_eq!(ids(&pool, ItemFilter::default()).await, vec![2]);
        });
    }
}
