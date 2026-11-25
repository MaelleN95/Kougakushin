// "Ce que je connais" : vocabulaire et grammaire actifs, lus pour les exercices d'IA.

use sqlx::SqlitePool;

use crate::conjugation::{Conjugation, VerbForm, VerbType, conjugate, known_forms};
use crate::particles::{self, KnownParticleUsage};

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub(crate) struct KnownWord {
    pub(crate) kana: String,
    pub(crate) kanji: Option<String>,
    // Sens dans l'ordre (table item_meaning), lus en tableau JSON.
    #[sqlx(json)]
    pub(crate) meanings: Vec<String>,
}

// Sens d'un mot en tableau JSON, dans l'ordre ; `i` = alias de la table item.
macro_rules! meanings_json {
    () => {
        "(SELECT json_group_array(m.meaning ORDER BY m.position)
          FROM item_meaning m WHERE m.item_id = i.id) AS meanings"
    };
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub(crate) struct KnownGrammar {
    pub(crate) title: String,
    pub(crate) explanation: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Knowledge {
    pub(crate) words: Vec<KnownWord>,
    pub(crate) grammar: Vec<KnownGrammar>,
    // 3 mots à placer en priorité, tirés au sort parmi les plus récents ou les plus oubliés.
    pub(crate) focus: Vec<KnownWord>,
    // Emplois de particules cochés "je connais" (D38 A), toutes leçons confondues.
    pub(crate) particles: Vec<KnownParticleUsage>,
    // Particules du catalogue dont aucun emploi n'est coché : signalées par le garde-fou.
    pub(crate) unknown_particles: Vec<String>,
    // Formes verbales que je connais (D41) : l'IA ne conjugue qu'avec elles.
    pub(crate) verb_forms: Vec<VerbForm>,
    // Tableaux de conjugaison de mes verbes actifs, pour le garde-fou (formes conjuguées reconnues).
    pub(crate) conjugations: Vec<Conjugation>,
}

// Filtre de leçons pour SQL : tableau JSON lu par json_each, ou NULL (= toutes les leçons).
fn lessons_json(lesson_ids: &[i64]) -> Option<String> {
    if lesson_ids.is_empty() {
        None
    } else {
        serde_json::to_string(lesson_ids).ok()
    }
}

// Éléments actifs des leçons choisies (toutes si `lesson_ids` est vide).
pub(crate) async fn find_knowledge(
    pool: &SqlitePool,
    lesson_ids: &[i64],
) -> Result<Knowledge, sqlx::Error> {
    let lessons = lessons_json(lesson_ids);

    let words = sqlx::query_as::<_, KnownWord>(concat!(
        "SELECT i.kana, i.kanji, ",
        meanings_json!(),
        r#"
        FROM item i
        WHERE i.kind = 'vocab' AND i.active = 1
          AND (?1 IS NULL OR i.lesson_id IN (SELECT value FROM json_each(?1)))
        ORDER BY i.id
        "#
    ))
    .bind(&lessons)
    .fetch_all(pool)
    .await?;

    let grammar = sqlx::query_as::<_, KnownGrammar>(
        r#"
        SELECT title, explanation FROM item
        WHERE kind = 'grammar' AND active = 1
          AND (?1 IS NULL OR lesson_id IN (SELECT value FROM json_each(?1)))
        ORDER BY id
        "#,
    )
    .bind(&lessons)
    .fetch_all(pool)
    .await?;

    // Priorité (SPEC.md 3.4) : parmi les 20 mots les plus oubliés (lapses) puis les plus récents,
    // 3 au hasard ; le hasard varie aussi les phrases d'un exercice à l'autre.
    let focus = sqlx::query_as::<_, KnownWord>(concat!(
        r#"
        SELECT kana, kanji, meanings FROM (
            SELECT i.kana, i.kanji, i.id, "#,
        meanings_json!(),
        r#",
                   COALESCE((SELECT MAX(c.lapses) FROM card c WHERE c.item_id = i.id), 0) AS lapses
            FROM item i
            WHERE i.kind = 'vocab' AND i.active = 1
              AND (?1 IS NULL OR i.lesson_id IN (SELECT value FROM json_each(?1)))
            ORDER BY lapses DESC, i.id DESC
            LIMIT 20
        )
        ORDER BY random()
        LIMIT 3
        "#
    ))
    .bind(&lessons)
    .fetch_all(pool)
    .await?;

    // Verbes actifs des mêmes leçons ; un verbe dont la terminaison ne correspond pas au type
    // est ignoré (signalé dans l'onglet Éléments).
    let verbs: Vec<(String, Option<String>, String)> = sqlx::query_as(
        r#"
        SELECT kana, kanji, word_type FROM item
        WHERE kind = 'vocab' AND active = 1
          AND word_type IN ('ru_verb', 'u_verb', 'irregular_verb')
          AND (?1 IS NULL OR lesson_id IN (SELECT value FROM json_each(?1)))
        ORDER BY id
        "#,
    )
    .bind(&lessons)
    .fetch_all(pool)
    .await?;
    let conjugations = verbs
        .iter()
        .filter_map(|(kana, kanji, word_type)| {
            let verb_type = VerbType::from_db(word_type)?;
            conjugate(kana, kanji.as_deref(), verb_type).ok()
        })
        .collect();

    Ok(Knowledge {
        words,
        grammar,
        focus,
        particles: particles::find_known_usages(pool).await?,
        unknown_particles: particles::find_unknown_particles(pool).await?,
        verb_forms: known_forms(pool).await?,
        conjugations,
    })
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::find_knowledge;
    use crate::conjugation::VerbForm;

    #[test]
    fn filtre_par_lecon_et_exclut_les_inactifs() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            sqlx::migrate!("./migrations").run(&pool).await.unwrap();
            sqlx::raw_sql(
                "INSERT INTO lesson (title, date) VALUES ('L1', '2026-11-18'), ('L2', '2026-11-25');
                 INSERT INTO item (lesson_id, kind, kana, kanji, meaning, word_type) VALUES (1, 'vocab', 'たべる', '食べる', 'manger', 'ru_verb');
                 INSERT INTO item (lesson_id, kind, active, kana, meaning) VALUES (1, 'vocab', 0, 'のむ', 'boire');
                 INSERT INTO item (lesson_id, kind, kana, meaning) VALUES (2, 'vocab', 'いく', 'aller');
                 INSERT INTO item (lesson_id, kind, title, explanation) VALUES (2, 'grammar', 'は', 'thème');
                 INSERT INTO item_meaning (item_id, position, meaning)
                     VALUES (1, 0, 'manger'), (2, 0, 'boire'), (3, 0, 'aller'), (3, 1, 'partir');",
            )
            .execute(&pool)
            .await
            .unwrap();

            let all = find_knowledge(&pool, &[]).await.unwrap();
            let kana: Vec<_> = all.words.iter().map(|w| w.kana.as_str()).collect();
            assert_eq!(kana, ["たべる", "いく"]); // のむ est inactif
            assert_eq!(all.grammar.len(), 1);
            assert_eq!(all.focus.len(), 2);

            let lesson_1 = find_knowledge(&pool, &[1]).await.unwrap();
            assert_eq!(lesson_1.words.len(), 1);
            assert_eq!(lesson_1.words[0].kanji.as_deref(), Some("食べる"));
            assert!(lesson_1.grammar.is_empty());
            assert_eq!(all.words[1].meanings, ["aller", "partir"]); // sens dans l'ordre

            // Particules : rien de coché au départ, puis un emploi de に.
            assert!(all.particles.is_empty());
            assert_eq!(all.unknown_particles.len(), 22);
            sqlx::query("UPDATE particle_usage SET active = 1 WHERE id = 11")
                .execute(&pool)
                .await
                .unwrap();
            let after = find_knowledge(&pool, &[]).await.unwrap();
            assert_eq!(after.particles.len(), 1);
            assert_eq!(after.particles[0].particle, "に");
            assert!(!after.unknown_particles.iter().any(|p| p == "に"));

            // Formes connues par défaut (migration) et tableau du seul verbe typé.
            assert_eq!(all.verb_forms, [VerbForm::Masu]);
            assert_eq!(all.conjugations.len(), 1);
            assert_eq!(all.conjugations[0][1].kanji.as_deref(), Some("食べます"));
            assert!(
                find_knowledge(&pool, &[2])
                    .await
                    .unwrap()
                    .conjugations
                    .is_empty()
            );
        });
    }
}
