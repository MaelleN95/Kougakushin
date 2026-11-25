// Accès SQL aux cartes.

use sqlx::{SqliteConnection, SqlitePool};

use super::model::{
    CardKey, Direction, FsrsRow, SessionRow, SessionScope, state_to_db, to_db_date, wanted_cards,
};
use crate::conjugation::{self, VerbForm, VerbType};
use crate::items::ItemContent;

// Les deux requêtes de session ne proposent que les cartes d'éléments actifs.
// `scope` : une leçon, un manuel entier, ou tout (les deux à None).

// Cartes à réviser : déjà vues, dont la date de prochaine révision est passée (`now` au format
// de la base). Les plus en retard d'abord.
pub(super) async fn find_to_review(
    pool: &SqlitePool,
    scope: SessionScope,
    now: &str,
) -> Result<Vec<SessionRow>, sqlx::Error> {
    sqlx::query_as::<_, SessionRow>(
        r#"
        SELECT c.id AS card_id, c.direction, c.form, c.state, i.word_type,
               i.kana, i.romaji, i.kanji, i.meaning, i.example_jp, i.example_fr
        FROM card c
        JOIN item i ON i.id = c.item_id
        WHERE i.active = 1
          AND (?1 IS NULL OR i.lesson_id = ?1)
          AND (?3 IS NULL OR i.lesson_id IN (SELECT id FROM lesson WHERE book_id = ?3))
          AND c.state <> 0
          AND c.next_review_at <= ?2
        ORDER BY c.next_review_at, c.id
        "#,
    )
    .bind(scope.lesson_id)
    .bind(now)
    .bind(scope.book_id)
    .fetch_all(pool)
    .await
}

// Cartes jamais vues, dans l'ordre de saisie, au plus `limit`.
pub(super) async fn find_new(
    pool: &SqlitePool,
    scope: SessionScope,
    limit: i64,
) -> Result<Vec<SessionRow>, sqlx::Error> {
    sqlx::query_as::<_, SessionRow>(
        r#"
        SELECT c.id AS card_id, c.direction, c.form, c.state, i.word_type,
               i.kana, i.romaji, i.kanji, i.meaning, i.example_jp, i.example_fr
        FROM card c
        JOIN item i ON i.id = c.item_id
        WHERE i.active = 1
          AND (?1 IS NULL OR i.lesson_id = ?1)
          AND (?3 IS NULL OR i.lesson_id IN (SELECT id FROM lesson WHERE book_id = ?3))
          AND c.state = 0
        ORDER BY i.id, c.id
        LIMIT ?2
        "#,
    )
    .bind(scope.lesson_id)
    .bind(limit)
    .bind(scope.book_id)
    .fetch_all(pool)
    .await
}

// Nombre de cartes jamais vues (éléments actifs), toutes limites confondues.
pub(super) async fn count_new(pool: &SqlitePool, scope: SessionScope) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM card c
        JOIN item i ON i.id = c.item_id
        WHERE i.active = 1 AND c.state = 0
          AND (?1 IS NULL OR i.lesson_id = ?1)
          AND (?2 IS NULL OR i.lesson_id IN (SELECT id FROM lesson WHERE book_id = ?2))
        "#,
    )
    .bind(scope.lesson_id)
    .bind(scope.book_id)
    .fetch_one(pool)
    .await
}

// Nombre de cartes nouvelles révisées pour la première fois depuis `since` (format de la base).
// review_log.state est l'état avant la révision : 0 = la carte était nouvelle.
pub(super) async fn count_new_reviewed_since(
    pool: &SqlitePool,
    since: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM review_log WHERE state = 0 AND reviewed_at >= ?")
        .bind(since)
        .fetch_one(pool)
        .await
}

// État FSRS d'une carte ; None si l'id n'existe pas.
pub(super) async fn find_fsrs_state(
    conn: &mut SqliteConnection,
    card_id: i64,
) -> Result<Option<FsrsRow>, sqlx::Error> {
    sqlx::query_as::<_, FsrsRow>(
        r#"
        SELECT next_review_at, stability, difficulty, elapsed_days, scheduled_days,
               reps, lapses, state, last_review
        FROM card
        WHERE id = ?
        "#,
    )
    .bind(card_id)
    .fetch_optional(conn)
    .await
}

// Enregistre le résultat d'une révision : nouvel état de la carte + ligne d'historique.
// À appeler dans une transaction (voir commands.rs).
pub(super) async fn save_review(
    conn: &mut SqliteConnection,
    card_id: i64,
    info: &rs_fsrs::SchedulingInfo,
) -> Result<(), sqlx::Error> {
    let card = &info.card;
    sqlx::query(
        r#"
        UPDATE card
        SET next_review_at = ?, stability = ?, difficulty = ?, elapsed_days = ?, scheduled_days = ?,
            reps = ?, lapses = ?, state = ?, last_review = ?
        WHERE id = ?
        "#,
    )
    .bind(to_db_date(card.due)) // `due` : nom du champ dans rs_fsrs
    .bind(card.stability)
    .bind(card.difficulty)
    .bind(card.elapsed_days)
    .bind(card.scheduled_days)
    .bind(card.reps)
    .bind(card.lapses)
    .bind(state_to_db(card.state))
    .bind(to_db_date(card.last_review))
    .bind(card_id)
    .execute(&mut *conn)
    .await?;

    let log = &info.review_log;
    sqlx::query(
        r#"
        INSERT INTO review_log (card_id, rating, state, elapsed_days, scheduled_days, reviewed_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(card_id)
    .bind(log.rating as i64) // 1 à 4, discriminants de rs_fsrs::Rating
    .bind(state_to_db(log.state))
    .bind(log.elapsed_days)
    .bind(log.scheduled_days)
    .bind(to_db_date(log.reviewed_date))
    .execute(&mut *conn)
    .await?;

    Ok(())
}

// Met les cartes d'un élément en accord avec son contenu :
// crée les cartes manquantes, supprime celles qui ne sont plus voulues, garde les autres
// (avec leur progression FSRS, ex. quand on corrige seulement la traduction).
//
// Reçoit une connexion et pas le pool : l'appelant l'utilise dans une transaction,
// pour que l'élément et ses cartes soient enregistrés ensemble ou pas du tout.
pub(crate) async fn sync_for_item(
    conn: &mut SqliteConnection,
    item_id: i64,
    content: &ItemContent,
) -> Result<(), sqlx::Error> {
    // &mut *conn : on prête la connexion pour cette requête seulement.
    let known_forms = conjugation::known_forms(&mut *conn).await?;
    let wanted = wanted_cards(content, &known_forms);

    // Cartes en trop (ou illisibles) : supprimées, avec leur historique (CASCADE).
    let existing: Vec<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, direction, form FROM card WHERE item_id = ?")
            .bind(item_id)
            .fetch_all(&mut *conn)
            .await?;
    for (id, direction, form) in existing {
        let key = CardKey::from_db(&direction, form.as_deref());
        if !key.is_some_and(|k| wanted.contains(&k)) {
            sqlx::query("DELETE FROM card WHERE id = ?")
                .bind(id)
                .execute(&mut *conn)
                .await?;
        }
    }

    for key in wanted {
        insert_card(&mut *conn, item_id, key).await?;
    }
    Ok(())
}

// Crée une carte si elle n'existe pas déjà.
// ON CONFLICT DO NOTHING sans cible : vaut pour l'index unique (item_id, direction, form), et
// seulement pour lui (un CHECK ou une clé étrangère violés restent des erreurs).
async fn insert_card(
    conn: &mut SqliteConnection,
    item_id: i64,
    key: CardKey,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO card (item_id, direction, form) VALUES (?, ?, ?) ON CONFLICT DO NOTHING",
    )
    .bind(item_id)
    .bind(key.direction.as_str())
    .bind(key.form.map(VerbForm::as_str))
    .execute(conn)
    .await?;
    Ok(())
}

// Une forme vient d'être activée ou désactivée : crée ses cartes pour tous les verbes conjugables,
// ou supprime toutes ses cartes (avec leur progression). À appeler dans la transaction qui
// change la forme (voir conjugation/commands.rs).
pub(crate) async fn sync_conjugation_form(
    conn: &mut SqliteConnection,
    form: VerbForm,
    known: bool,
) -> Result<(), sqlx::Error> {
    if !known {
        sqlx::query("DELETE FROM card WHERE direction = ? AND form = ?")
            .bind(Direction::Conjugation.as_str())
            .bind(form.as_str())
            .execute(&mut *conn)
            .await?;
        return Ok(());
    }

    let verbs: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, kana, word_type FROM item
         WHERE kind = 'vocab' AND word_type IN ('ru_verb', 'u_verb', 'irregular_verb')",
    )
    .fetch_all(&mut *conn)
    .await?;
    for (item_id, kana, word_type) in verbs {
        let conjugable = VerbType::from_db(&word_type)
            .is_some_and(|t| conjugation::conjugate(&kana, None, t).is_ok());
        if conjugable {
            insert_card(&mut *conn, item_id, CardKey::conjugation(form)).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::conjugation::{self, VerbForm, VerbType};
    use crate::items::{ItemContent, WordType};

    // Base en mémoire (une seule connexion) avec une leçon et un élément vide de cartes (id 1).
    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query("INSERT INTO lesson (title, date) VALUES ('L1', '2026-11-18')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (1, 'vocab', 'た', 'x')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn vocab(kanji: Option<&str>) -> ItemContent {
        ItemContent::Vocab {
            kana: "た".into(),
            romaji: None,
            kanji: kanji.map(String::from),
            meaning: "x".into(),
            word_type: None,
        }
    }

    // (id, direction) des cartes de l'élément 1, triées par id.
    async fn cards(pool: &SqlitePool) -> Vec<(i64, String)> {
        sqlx::query_as("SELECT id, direction FROM card WHERE item_id = 1 ORDER BY id")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    async fn sync(pool: &SqlitePool, content: &ItemContent) {
        let mut conn = pool.acquire().await.unwrap();
        super::sync_for_item(&mut conn, 1, content).await.unwrap();
    }

    #[test]
    fn cree_ajoute_garde_et_supprime_les_cartes() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;

            sync(&pool, &vocab(None)).await;
            let first = cards(&pool).await;
            assert_eq!(first.len(), 2);

            // Ajout d'un kanji : une carte de plus, les deux premières sont conservées.
            sync(&pool, &vocab(Some("田"))).await;
            let with_kanji = cards(&pool).await;
            assert_eq!(with_kanji.len(), 3);
            assert_eq!(with_kanji[..2], first[..]);

            // Relancer sans changement ne crée rien.
            sync(&pool, &vocab(Some("田"))).await;
            assert_eq!(cards(&pool).await, with_kanji);

            // Kanji retiré : seule la carte kanji disparaît.
            sync(&pool, &vocab(None)).await;
            assert_eq!(cards(&pool).await, first);

            // Devenu grammaire : plus aucune carte.
            let grammar = ItemContent::Grammar {
                title: "t".into(),
                explanation: "e".into(),
            };
            sync(&pool, &grammar).await;
            assert!(cards(&pool).await.is_empty());
        });
    }

    #[test]
    fn la_migration_cree_les_cartes_des_elements_existants() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            // Base dans l'état d'avant la phase 5 : leçon + éléments, sans table card.
            sqlx::raw_sql(include_str!(
                "../../migrations/20261120000000_create_lesson.sql"
            ))
            .execute(&pool)
            .await
            .unwrap();
            sqlx::raw_sql(include_str!(
                "../../migrations/20261122000000_create_item.sql"
            ))
            .execute(&pool)
            .await
            .unwrap();
            sqlx::raw_sql(
                "INSERT INTO lesson (title, date) VALUES ('L1', '2026-11-18');
                 INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (1, 'vocab', 'た', 'x');
                 INSERT INTO item (lesson_id, kind, kana, kanji, meaning) VALUES (1, 'vocab', 'た', '田', 'x');
                 INSERT INTO item (lesson_id, kind, title, explanation) VALUES (1, 'grammar', 't', 'e');",
            )
            .execute(&pool)
            .await
            .unwrap();

            sqlx::raw_sql(include_str!(
                "../../migrations/20261122100000_create_card.sql"
            ))
            .execute(&pool)
            .await
            .unwrap();

            let per_item: Vec<(i64, i64)> = sqlx::query_as(
                "SELECT item_id, COUNT(*) FROM card GROUP BY item_id ORDER BY item_id",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            // 2 cartes sans kanji, 3 avec, aucune pour la grammaire.
            assert_eq!(per_item, vec![(1, 2), (2, 3)]);
        });
    }

    #[test]
    fn supprimer_l_element_supprime_ses_cartes_et_leur_historique() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            sync(&pool, &vocab(None)).await;
            sqlx::query(
                "INSERT INTO review_log (card_id, rating, state, elapsed_days, scheduled_days, reviewed_at)
                 SELECT id, 3, 0, 0, 1, datetime('now') FROM card",
            )
            .execute(&pool)
            .await
            .unwrap();

            sqlx::query("DELETE FROM item WHERE id = 1")
                .execute(&pool)
                .await
                .unwrap();

            let cards_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM card")
                .fetch_one(&pool)
                .await
                .unwrap();
            let logs_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM review_log")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!((cards_left, logs_left), (0, 0));
        });
    }

    fn typed_verb(kana: &str, word_type: WordType) -> ItemContent {
        ItemContent::Vocab {
            kana: kana.into(),
            romaji: None,
            kanji: None,
            meaning: "x".into(),
            word_type: Some(word_type),
        }
    }

    // (direction, form) des cartes de l'élément 1, triées par id.
    async fn card_keys(pool: &SqlitePool) -> Vec<(String, Option<String>)> {
        sqlx::query_as("SELECT direction, form FROM card WHERE item_id = 1 ORDER BY id")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    async fn set_form(pool: &SqlitePool, form: VerbForm, known: bool) {
        let mut conn = pool.acquire().await.unwrap();
        sqlx::query("UPDATE conjugation_form SET known = ? WHERE form = ?")
            .bind(known)
            .bind(form.as_str())
            .execute(&mut *conn)
            .await
            .unwrap();
        super::sync_conjugation_form(&mut conn, form, known)
            .await
            .unwrap();
    }

    #[test]
    fn cartes_de_conjugaison_selon_les_formes_connues() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            // sync_conjugation_form lit la table item : l'élément 1 y devient le verbe たべる.
            sqlx::query("UPDATE item SET kana = 'たべる', word_type = 'ru_verb' WHERE id = 1")
                .execute(&pool)
                .await
                .unwrap();
            let verb = typed_verb("たべる", WordType::RuVerb);
            let masu = ("conjugation".to_string(), Some("masu".to_string()));
            let te = ("conjugation".to_string(), Some("te".to_string()));

            // ます connue par défaut (migration) : fr_jp, jp_fr, ます.
            sync(&pool, &verb).await;
            let first = card_keys(&pool).await;
            assert_eq!(first.len(), 3);
            assert_eq!(first[2], masu);
            // Relancer ne duplique rien (index unique avec ifnull(form, '')).
            sync(&pool, &verb).await;
            assert_eq!(card_keys(&pool).await, first);

            // Activer て : une carte de plus pour ce verbe ; désactiver ます : sa carte disparaît.
            set_form(&pool, VerbForm::Te, true).await;
            assert_eq!(card_keys(&pool).await.last(), Some(&te));
            set_form(&pool, VerbForm::Masu, false).await;
            assert!(!card_keys(&pool).await.contains(&masu));
            assert_eq!(card_keys(&pool).await.len(), 3);

            // Le mot redevient un nom : plus de carte de conjugaison.
            sync(&pool, &typed_verb("たべる", WordType::Noun)).await;
            assert_eq!(card_keys(&pool).await.len(), 2);
            // Verbe dont la terminaison ne correspond pas au type : aucune carte de conjugaison.
            sync(&pool, &typed_verb("たべ", WordType::RuVerb)).await;
            assert_eq!(card_keys(&pool).await.len(), 2);
            sqlx::query("UPDATE item SET kana = 'たべ' WHERE id = 1")
                .execute(&pool)
                .await
                .unwrap();
            set_form(&pool, VerbForm::Nai, true).await;
            assert_eq!(card_keys(&pool).await.len(), 2);
        });
    }

    #[test]
    fn index_unique_refuse_les_doublons_et_check_exige_une_forme() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let insert = |direction: &'static str, form: Option<&'static str>| {
                let pool = pool.clone();
                async move {
                    sqlx::query("INSERT INTO card (item_id, direction, form) VALUES (1, ?, ?)")
                        .bind(direction)
                        .bind(form)
                        .execute(&pool)
                        .await
                }
            };
            assert!(insert("fr_jp", None).await.is_ok());
            assert!(insert("fr_jp", None).await.is_err()); // deux NULL : doublon quand même
            assert!(insert("conjugation", Some("te")).await.is_ok());
            assert!(insert("conjugation", Some("te")).await.is_err());
            assert!(insert("conjugation", Some("ta")).await.is_ok());
            assert!(insert("conjugation", None).await.is_err()); // CHECK : forme obligatoire
            assert!(insert("jp_fr", Some("te")).await.is_err()); // CHECK : forme interdite
        });
    }

    // Base dans l'état d'avant la migration 20261126000200 (toutes les migrations précédentes).
    async fn pool_before_conjugation() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for sql in [
            include_str!("../../migrations/20261120000000_create_lesson.sql"),
            include_str!("../../migrations/20261122000000_create_item.sql"),
            include_str!("../../migrations/20261122100000_create_card.sql"),
            include_str!("../../migrations/20261123000000_rename_card_due.sql"),
            include_str!("../../migrations/20261123000100_create_setting.sql"),
            include_str!("../../migrations/20261124000000_create_book.sql"),
        ] {
            sqlx::raw_sql(sql).execute(&pool).await.unwrap();
        }
        pool
    }

    // Toutes les colonnes d'une carte d'avant la migration, en texte, pour comparer avant / après.
    const CARD_COLUMNS: &str =
        "SELECT id || '|' || item_id || '|' || direction || '|' || next_review_at
        || '|' || stability || '|' || difficulty || '|' || elapsed_days || '|' || scheduled_days
        || '|' || reps || '|' || lapses || '|' || state || '|' || ifnull(last_review, '-')
        || '|' || created_at FROM card WHERE direction <> 'conjugation' ORDER BY id";
    const LOG_COLUMNS: &str = "SELECT id || '|' || card_id || '|' || rating || '|' || state
        || '|' || elapsed_days || '|' || scheduled_days || '|' || reviewed_at
        FROM review_log ORDER BY id";

    async fn texts(pool: &SqlitePool, sql: &'static str) -> Vec<String> {
        sqlx::query_scalar(sql).fetch_all(pool).await.unwrap()
    }

    #[test]
    fn la_reconstruction_de_card_garde_les_cartes_fsrs_et_l_historique() {
        tauri::async_runtime::block_on(async {
            let pool = pool_before_conjugation().await;
            // Éléments : verbes valides et invalides, un nom. Le rattrapage de la migration
            // 20261122100000 a déjà tourné (table vide) : les cartes sont créées à la main.
            sqlx::raw_sql(
                "INSERT INTO lesson (title, date) VALUES ('L1', '2026-11-18');
                 INSERT INTO item (lesson_id, kind, kana, kanji, meaning, word_type) VALUES
                    (1, 'vocab', 'たべる', '食べる', 'manger', 'ru_verb'),
                    (1, 'vocab', 'たべ', NULL, 'x', 'ru_verb'),
                    (1, 'vocab', 'かう', NULL, 'acheter', 'u_verb'),
                    (1, 'vocab', 'いけ', NULL, 'x', 'u_verb'),
                    (1, 'vocab', 'べんきょうする', NULL, 'étudier', 'irregular_verb'),
                    (1, 'vocab', 'たべる', NULL, 'x', 'irregular_verb'),
                    (1, 'vocab', 'する', NULL, 'faire', 'irregular_verb'),
                    (1, 'vocab', 'る', NULL, 'x', 'ru_verb'),
                    (1, 'vocab', 'ほん', NULL, 'livre', 'noun'),
                    (1, 'vocab', 'のむ', NULL, 'boire', NULL);
                 INSERT INTO card (item_id, direction) SELECT id, 'fr_jp' FROM item;
                 INSERT INTO card (item_id, direction) SELECT id, 'jp_fr' FROM item;
                 INSERT INTO card (item_id, direction) VALUES (1, 'kanji_read');
                 UPDATE card SET state = 2, reps = 4, lapses = 1, stability = 3.25,
                     difficulty = 5.5, elapsed_days = 2, scheduled_days = 6,
                     next_review_at = '2026-11-30 08:00:00', last_review = '2026-11-24 08:00:00'
                 WHERE id IN (1, 12);
                 INSERT INTO review_log (card_id, rating, state, elapsed_days, scheduled_days, reviewed_at)
                     VALUES (1, 3, 0, 0, 1, '2026-11-20 10:00:00'),
                            (1, 4, 2, 2, 6, '2026-11-24 08:00:00'),
                            (12, 1, 0, 0, 0, '2026-11-24 08:00:00');
                 -- Carte supprimée : le compteur AUTOINCREMENT est plus haut que le plus grand id.
                 INSERT INTO card (item_id, direction) VALUES (2, 'kanji_read');
                 DELETE FROM card WHERE id = 22;",
            )
            .execute(&pool)
            .await
            .unwrap();
            let cards_before = texts(&pool, CARD_COLUMNS).await;
            let logs_before = texts(&pool, LOG_COLUMNS).await;
            assert_eq!((cards_before.len(), logs_before.len()), (21, 3));

            // Comme sqlx : dans une transaction, clés étrangères activées.
            let mut tx = pool.begin().await.unwrap();
            sqlx::raw_sql(include_str!(
                "../../migrations/20261126000200_rebuild_card_for_conjugation.sql"
            ))
            .execute(&mut *tx)
            .await
            .unwrap();
            tx.commit().await.unwrap();

            // Cartes, état FSRS et historique identiques, id compris.
            assert_eq!(texts(&pool, CARD_COLUMNS).await, cards_before);
            assert_eq!(texts(&pool, LOG_COLUMNS).await, logs_before);
            let orphans: Vec<(String, i64, String, i64)> =
                sqlx::query_as("PRAGMA foreign_key_check")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            assert!(orphans.is_empty());

            // Cartes ます créées pour les verbes conjugables, selon la même règle qu'en Rust.
            let masu_items: Vec<i64> = sqlx::query_scalar(
                "SELECT item_id FROM card WHERE direction = 'conjugation' AND form = 'masu'
                 ORDER BY item_id",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            assert_eq!(masu_items, [1, 3, 5, 7]);
            let items: Vec<(i64, String, Option<String>)> =
                sqlx::query_as("SELECT id, kana, word_type FROM item ORDER BY id")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            for (id, kana, word_type) in items {
                let rust_ok = word_type
                    .as_deref()
                    .and_then(VerbType::from_db)
                    .is_some_and(|t| conjugation::conjugate(&kana, None, t).is_ok());
                assert_eq!(rust_ok, masu_items.contains(&id), "élément {id} ({kana})");
            }

            // Compteur conservé : la carte suivante ne reprend pas l'id 22 d'une carte supprimée.
            sqlx::query("INSERT INTO card (item_id, direction) VALUES (9, 'kanji_read')")
                .execute(&pool)
                .await
                .unwrap();
            let last_id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM card")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(last_id, 22 + masu_items.len() as i64 + 1);

            // La clé étrangère de review_log pointe vers la nouvelle table : la cascade marche.
            sqlx::query("DELETE FROM item WHERE id = 1")
                .execute(&pool)
                .await
                .unwrap();
            let logs_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM review_log")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(logs_left, 1);
        });
    }
}
