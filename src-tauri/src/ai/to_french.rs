// Traduction japonais => français fiable pour des phrases écrites en kana.
// Le traducteur (TranslateGemma) se trompe sur les phrases tout en hiragana avec espaces :
// "わたし は マレーシア の ちゅうごくご が いい です" devenait "J'apprécie beaucoup la culture
// malaisienne". Le modèle principal réécrit donc d'abord la phrase en écriture normale
// (私はマレーシアの中国語がいいです), puis le traducteur traduit cette version.

use serde_json::json;

use super::client::{ChatRequest, ChatTurn, Ollama};
use crate::error::AppError;

#[derive(serde::Deserialize)]
struct Rewritten {
    japanese: String,
}

// Vrai si les kana de `rewritten` apparaissent tous, dans l'ordre, dans `original`.
// Les kanji ajoutés remplacent des kana : la réécriture ne peut pas ajouter de kana.
// Contrôle simple qui écarte une réécriture qui aurait changé de phrase.
// Seuls les kana comptent : espaces et ponctuation ajoutés (。) sont sans effet sur le sens.
fn keeps_kana(original: &str, rewritten: &str) -> bool {
    // Hiragana et katakana (ー compris).
    let is_kana = |c: &char| ('\u{3041}'..='\u{30FF}').contains(c) && *c != '・';
    let mut source = original.chars().filter(is_kana);
    rewritten
        .chars()
        .filter(is_kana)
        .all(|c| source.any(|o| o == c))
}

// Phrase en écriture normale (kanji usuels, sans espaces) ; la phrase d'origine sans espaces
// si la réécriture échoue ou change la phrase.
async fn with_kanji(ollama: &Ollama, model: &str, japanese: &str) -> String {
    let fallback: String = japanese.chars().filter(|c| !c.is_whitespace()).collect();
    let result: Result<Rewritten, AppError> = ollama
        .chat_json(ChatRequest {
            model,
            system: "Réécris la phrase japonaise donnée en écriture japonaise normale (kanji \
                     usuels, sans espaces), sans changer un seul mot, ni la conjugaison, ni le \
                     sens. Garde la ponctuation.",
            turns: &[ChatTurn::user(japanese)],
            schema: json!({
                "type": "object",
                "properties": { "japanese": { "type": "string" } },
                "required": ["japanese"]
            }),
            temperature: 0.0,
        })
        .await;
    match result {
        Ok(r) if !r.japanese.trim().is_empty() && keeps_kana(japanese, &r.japanese) => {
            r.japanese.trim().to_string()
        }
        Ok(r) => {
            log::warn!(
                "Réécriture en kanji écartée : {} => {}",
                japanese,
                r.japanese
            );
            fallback
        }
        Err(e) => {
            log::warn!("Réécriture en kanji impossible : {e}");
            fallback
        }
    }
}

// `model` réécrit en kanji, `translator` traduit.
pub(super) async fn to_french(
    ollama: &Ollama,
    model: &str,
    translator: &str,
    japanese: &str,
) -> Result<String, AppError> {
    let written = with_kanji(ollama, model, japanese).await;
    ollama.translate_ja_to_fr(translator, &written).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reecriture_qui_garde_les_kana() {
        let original = "わたし は マレーシア の ちゅうごくご が いい です";
        assert!(keeps_kana(original, "私はマレーシアの中国語がいいです"));
        // Point final ajouté par le modèle (cas réel) : accepté.
        assert!(keeps_kana(original, "私はマレーシアの中国語がいいです。"));
        assert!(keeps_kana(
            "あした がっこう に いきます",
            "明日学校に行きます"
        ));
    }

    #[test]
    fn reecriture_qui_change_la_phrase() {
        let original = "わたし は マレーシア の ちゅうごくご が いい です";
        // Particule changée (が => を) et mot ajouté (とても).
        assert!(!keeps_kana(
            original,
            "私はマレーシアの中国語をとても好きです"
        ));
        // Négation ajoutée.
        assert!(!keeps_kana("あした いきます", "明日行きません"));
    }

    // Essai réel (Ollama lancé, modèles installés) : cargo test essai_reel_traduction -- --ignored
    #[test]
    #[ignore]
    fn essai_reel_traduction_d_une_phrase_en_kana() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let french = to_french(
                &ollama,
                &super::super::knowledge_prompt::test_model(),
                "translategemma:4b",
                "わたし は マレーシア の ちゅうごくご が いい です",
            )
            .await
            .unwrap();
            println!("{french}");
            assert!(!french.contains("culture"), "traduction fausse : {french}");
        });
    }
}
