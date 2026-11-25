// Commandes Tauri de la conjugaison : tableau d'un verbe, formes connues, verbes à vérifier.

use sqlx::SqlitePool;
use tauri::State;

use super::model::{Conjugation, VerbForm, VerbIssue, VerbType, conjugate, verb_issue};
use super::repository;
use crate::cards;
use crate::error::AppError;

// Une forme et son interrupteur "je connais / pas encore".
#[derive(Debug, serde::Serialize)]
pub(crate) struct ConjugationFormState {
    form: VerbForm,
    known: bool,
}

// Mot de vocabulaire signalé dans l'onglet Éléments.
#[derive(Debug, serde::Serialize)]
pub(crate) struct VerbWarning {
    item_id: i64,
    issue: VerbIssue,
}

async fn form_states(pool: &SqlitePool) -> Result<Vec<ConjugationFormState>, AppError> {
    let known = repository::known_forms(pool).await?;
    Ok(VerbForm::ALL
        .into_iter()
        .map(|form| ConjugationFormState {
            form,
            known: known.contains(&form),
        })
        .collect())
}

// JS : invoke("conjugate_verb", { kana: "たべる", kanji: "食べる", verbType: "ru_verb" }).
// Sans accès à la base : sert à l'aperçu en direct du formulaire et au tableau de l'onglet Éléments.
#[tauri::command]
pub(crate) fn conjugate_verb(
    kana: String,
    kanji: Option<String>,
    verb_type: VerbType,
) -> Result<Conjugation, AppError> {
    conjugate(&kana, kanji.as_deref(), verb_type).map_err(AppError::Validation)
}

#[tauri::command]
pub(crate) async fn list_conjugation_forms(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<ConjugationFormState>, AppError> {
    form_states(pool.inner()).await
}

// Active ou désactive une forme, et crée ou supprime ses cartes de conjugaison dans la même
// transaction. Désactiver supprime les cartes de la forme et leur progression (confirmé à l'écran).
#[tauri::command]
pub(crate) async fn set_conjugation_form_known(
    pool: State<'_, SqlitePool>,
    form: VerbForm,
    known: bool,
) -> Result<Vec<ConjugationFormState>, AppError> {
    let mut tx = pool.begin().await?;
    repository::set_known(&mut tx, form, known).await?;
    cards::sync_conjugation_form(&mut tx, form, known).await?;
    tx.commit().await?;
    form_states(pool.inner()).await
}

// Verbes au type manquant ("type de verbe à préciser") ou incompatible avec leur terminaison.
#[tauri::command]
pub(crate) async fn list_verb_warnings(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<VerbWarning>, AppError> {
    let rows = repository::find_vocab(pool.inner()).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            verb_issue(&row.kana, &row.meaning, row.word_type.as_deref()).map(|issue| VerbWarning {
                item_id: row.id,
                issue,
            })
        })
        .collect())
}
