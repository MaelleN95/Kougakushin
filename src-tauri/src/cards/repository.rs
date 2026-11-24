// Accès SQL aux cartes.

use sqlx::{SqliteConnection, SqlitePool};

use super::model::{
    Direction, FsrsRow, SessionRow, SessionScope, directions_for, state_to_db, to_db_date,
};
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
        SELECT c.id AS card_id, c.direction, c.state,
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
        SELECT c.id AS card_id, c.direction, c.state,
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
    let wanted = directions_for(content);

    for direction in Direction::ALL {
        if wanted.contains(&direction) {
            // ON CONFLICT DO NOTHING : la carte existe déjà (UNIQUE item_id + direction).
            sqlx::query(
                "INSERT INTO card (item_id, direction) VALUES (?, ?)
                 ON CONFLICT (item_id, direction) DO NOTHING",
            )
            .bind(item_id)
            .bind(direction.as_str())
            .execute(&mut *conn) // &mut *conn : on prête la connexion pour cette requête seulement
            .await?;
        } else {
            sqlx::query("DELETE FROM card WHERE item_id = ? AND direction = ?")
                .bind(item_id)
                .bind(direction.as_str())
                .execute(&mut *conn)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::items::ItemContent;

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
}
