// Commandes Tauri de révision : constituer une session, enregistrer une note.
// La logique est dans des fonctions qui reçoivent `now`, pour être testable à une date fixée.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tauri::State;

use super::model::{
    ReviewRating, ReviewResult, ReviewSession, SessionRow, SessionScope, start_of_local_day,
    to_db_date,
};
use super::repository;
use crate::error::AppError;
use crate::settings;

// Cartes à réviser maintenant, puis les nouvelles dans la limite du jour (réglage),
// augmentée de `extra_new` quand l'utilisatrice demande "N nouvelles de plus".
pub(super) async fn build_session(
    pool: &SqlitePool,
    scope: SessionScope,
    extra_new: i64,
    now: DateTime<Utc>,
) -> Result<ReviewSession, AppError> {
    if extra_new < 0 {
        return Err(AppError::Validation(
            "Le nombre de cartes supplémentaires ne peut pas être négatif",
        ));
    }
    let to_review = repository::find_to_review(pool, scope, &to_db_date(now)).await?;

    let per_day = settings::load(pool).await?.new_cards_per_day;
    let since = to_db_date(start_of_local_day(now));
    let already_new = repository::count_new_reviewed_since(pool, &since).await?;
    let remaining_new = (per_day + extra_new - already_new).max(0);
    let new = repository::find_new(pool, scope, remaining_new).await?;

    // Reste-t-il des nouvelles cartes au-delà de celles proposées ? (bouton "N de plus")
    let more_new_available = repository::count_new(pool, scope).await? > new.len() as i64;

    // chain : les cartes à réviser d'abord, puis les nouvelles.
    // flatten : écarte les None (cartes de conjugaison d'un verbe non conjugable).
    let cards = to_review
        .into_iter()
        .chain(new)
        .map(SessionRow::into_review_card)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(ReviewSession {
        cards,
        more_new_available,
    })
}

// Planifie la carte avec FSRS selon la note, puis enregistre le tout en une transaction.
pub(super) async fn apply_review(
    pool: &SqlitePool,
    card_id: i64,
    rating: ReviewRating,
    now: DateTime<Utc>,
) -> Result<ReviewResult, AppError> {
    let mut tx = pool.begin().await?;

    let row = repository::find_fsrs_state(&mut tx, card_id)
        .await?
        .ok_or(AppError::NotFound("Carte"))?;
    let info = rs_fsrs::FSRS::default().next(row.to_fsrs(now)?, now, rating.to_fsrs());
    repository::save_review(&mut tx, card_id, &info).await?;

    tx.commit().await?;
    Ok(ReviewResult {
        next_review_at: to_db_date(info.card.due), // `due` : nom du champ dans rs_fsrs
    })
}

// JS : invoke("get_review_session", { lessonId: 3, bookId: null, extraNew: 10 }) ; tous facultatifs.
// lessonId et bookId ensemble : refusé, la session porte sur l'un ou l'autre.
#[tauri::command]
pub(crate) async fn get_review_session(
    pool: State<'_, SqlitePool>,
    lesson_id: Option<i64>,
    book_id: Option<i64>,
    extra_new: Option<i64>,
) -> Result<ReviewSession, AppError> {
    if lesson_id.is_some() && book_id.is_some() {
        return Err(AppError::Validation(
            "Choisis une leçon ou un manuel, pas les deux",
        ));
    }
    let scope = SessionScope { lesson_id, book_id };
    build_session(pool.inner(), scope, extra_new.unwrap_or(0), Utc::now()).await
}

// JS : invoke("review_card", { cardId: 12, rating: "good" }).
#[tauri::command]
pub(crate) async fn review_card(
    pool: State<'_, SqlitePool>,
    card_id: i64,
    rating: ReviewRating,
) -> Result<ReviewResult, AppError> {
    apply_review(pool.inner(), card_id, rating, Utc::now()).await
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, TimeZone, Utc};
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::super::model::{ReviewRating, SessionScope};
    use super::{apply_review, build_session};
    use crate::error::AppError;

    // Midi UTC : loin de minuit, le "jour" local est le même en France.
    fn noon() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 22, 12, 0, 0).unwrap()
    }

    // Base en mémoire : `count` mots actifs (2 cartes nouvelles chacun) + 1 mot inactif.
    async fn pool_with_words(count: i64) -> SqlitePool {
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
        for n in 0..=count {
            let active = if n < count { 1 } else { 0 }; // le dernier est inactif
            sqlx::query(
                "INSERT INTO item (lesson_id, kind, active, kana, meaning) VALUES (1, 'vocab', ?, 'た', 'x')",
            )
            .bind(active)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO card (item_id, direction) SELECT id, 'fr_jp' FROM item;
             INSERT INTO card (item_id, direction) SELECT id, 'jp_fr' FROM item;",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    // Ids des cartes proposées, dans l'ordre.
    async fn session_ids(pool: &SqlitePool, now: DateTime<Utc>) -> Vec<i64> {
        build_session(pool, SessionScope::default(), 0, now)
            .await
            .unwrap()
            .cards
            .into_iter()
            .map(|card| {
                serde_json::to_value(card).unwrap()["card_id"]
                    .as_i64()
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn nouvelles_cartes_limitees_a_10_par_jour_et_elements_inactifs_exclus() {
        tauri::async_runtime::block_on(async {
            let pool = pool_with_words(6).await; // 12 cartes actives, 2 inactives
            let ids = session_ids(&pool, noon()).await;
            assert_eq!(ids.len(), 10);
            // Les cartes du mot inactif (item 7 => cartes 7 et 14) ne sont jamais proposées.
            assert!(!ids.contains(&7) && !ids.contains(&14));
        });
    }

    #[test]
    fn une_revision_planifie_la_carte_et_consomme_le_quota_du_jour() {
        tauri::async_runtime::block_on(async {
            let pool = pool_with_words(6).await;
            let result = apply_review(&pool, 1, ReviewRating::Good, noon())
                .await
                .unwrap();
            assert!(
                serde_json::to_value(&result).unwrap()["next_review_at"]
                    .as_str()
                    .unwrap()
                    > "2026-11-22 12:00:00"
            );

            let (state, reps): (i64, i64) =
                sqlx::query_as("SELECT state, reps FROM card WHERE id = 1")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_ne!(state, 0); // n'est plus nouvelle
            assert_eq!(reps, 1);
            let logs: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM review_log WHERE card_id = 1 AND state = 0",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(logs, 1);

            // Une nouvelle carte vue aujourd'hui : il en reste 9 à proposer, sans la carte 1.
            let ids = session_ids(&pool, noon() + Duration::seconds(1)).await;
            assert_eq!(ids.len(), 9);
            assert!(!ids.contains(&1));
        });
    }

    #[test]
    fn a_revoir_ramene_la_carte_dans_les_cartes_dues() {
        tauri::async_runtime::block_on(async {
            let pool = pool_with_words(1).await;
            apply_review(&pool, 1, ReviewRating::Again, noon())
                .await
                .unwrap();
            // Le lendemain, elle est à réviser : proposée en premier, avant les nouvelles.
            let ids = session_ids(&pool, noon() + Duration::days(1)).await;
            assert_eq!(ids.first(), Some(&1));
        });
    }

    #[test]
    fn bouton_n_nouvelles_de_plus() {
        tauri::async_runtime::block_on(async {
            let pool = pool_with_words(6).await; // 12 nouvelles cartes actives
            let session = build_session(&pool, SessionScope::default(), 0, noon())
                .await
                .unwrap();
            assert_eq!(session.cards.len(), 10);
            assert!(session.more_new_available);

            let more = build_session(&pool, SessionScope::default(), 10, noon())
                .await
                .unwrap();
            assert_eq!(more.cards.len(), 12); // limite 20, seulement 12 disponibles
            assert!(!more.more_new_available);

            assert!(
                build_session(&pool, SessionScope::default(), -1, noon())
                    .await
                    .is_err()
            );
        });
    }

    #[test]
    fn session_limitee_a_une_lecon_ou_a_un_manuel() {
        tauri::async_runtime::block_on(async {
            let pool = pool_with_words(2).await; // leçon 1 sans manuel : 4 cartes actives
            // Manuel 1 > leçon 2 > un mot actif (item 4) et ses 2 cartes.
            sqlx::raw_sql(
                "INSERT INTO book (title) VALUES ('Marugoto A1');
                 INSERT INTO lesson (title, date, book_id) VALUES ('L2', '2026-11-19', 1);
                 INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (2, 'vocab', 'み', 'y');
                 INSERT INTO card (item_id, direction) VALUES (4, 'fr_jp'), (4, 'jp_fr');",
            )
            .execute(&pool)
            .await
            .unwrap();

            let count = |scope: SessionScope| {
                let pool = pool.clone();
                async move {
                    build_session(&pool, scope, 0, noon())
                        .await
                        .unwrap()
                        .cards
                        .len()
                }
            };
            let book = SessionScope {
                lesson_id: None,
                book_id: Some(1),
            };
            let lesson = SessionScope {
                lesson_id: Some(1),
                book_id: None,
            };
            assert_eq!(count(book).await, 2);
            assert_eq!(count(lesson).await, 4);
            assert_eq!(count(SessionScope::default()).await, 6);
        });
    }

    #[test]
    fn carte_de_conjugaison_dans_la_session() {
        tauri::async_runtime::block_on(async {
            let pool = pool_with_words(0).await; // un seul mot, inactif : aucune carte proposée
            // Verbe たべる (食べる) avec sa carte ます, et un "verbe" non conjugable avec une carte
            // ます incohérente (ex. fichier importé) : écartée sans bloquer la session.
            sqlx::raw_sql(
                "INSERT INTO item (lesson_id, kind, kana, kanji, meaning, word_type)
                     VALUES (1, 'vocab', 'たべる', '食べる', 'manger', 'ru_verb'),
                            (1, 'vocab', 'たべ', NULL, 'x', 'ru_verb');
                 INSERT INTO card (item_id, direction, form)
                     VALUES (2, 'conjugation', 'masu'), (3, 'conjugation', 'masu');",
            )
            .execute(&pool)
            .await
            .unwrap();

            let session = build_session(&pool, SessionScope::default(), 0, noon())
                .await
                .unwrap();
            assert_eq!(session.cards.len(), 1);
            let card = serde_json::to_value(&session.cards[0]).unwrap();
            assert_eq!(card["direction"], "conjugation");
            assert_eq!(card["conjugation"]["form"], "masu");
            assert_eq!(card["conjugation"]["question_kanji"], "食べる");
            assert_eq!(card["conjugation"]["answer_kana"], "たべます");
            assert_eq!(card["conjugation"]["answer_kanji"], "食べます");
        });
    }

    #[test]
    fn carte_inconnue() {
        tauri::async_runtime::block_on(async {
            let pool = pool_with_words(1).await;
            let error = apply_review(&pool, 999, ReviewRating::Good, noon()).await;
            assert!(matches!(error, Err(AppError::NotFound("Carte"))));
        });
    }
}
