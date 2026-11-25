// Traduction japonais => français fiable pour des phrases écrites en kana.
// Le traducteur (TranslateGemma) se trompe sur les phrases tout en hiragana avec espaces :
// "わたし は マレーシア の ちゅうごくご が いい です" devenait "J'apprécie beaucoup la culture
// malaisienne". Le modèle principal réécrit donc d'abord la phrase en écriture normale
// (私はマレーシアの中国語がいいです), puis le traducteur traduit cette version.

use serde_json::json;

use super::client::{ChatRequest, ChatTurn, Ollama};
use super::knowledge_prompt::is_kanji;
use crate::error::AppError;
use crate::items::Knowledge;

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
pub(super) async fn with_kanji(ollama: &Ollama, model: &str, japanese: &str) -> String {
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

// Formes sous lesquelles un mot peut apparaître dans une phrase : tel quel (espaces retirés),
// en kanji, ou par son radical s'il se conjugue (たべ de たべる, 食 de 食べる).
// Radical en kana d'au moins 2 caractères : の (de のむ) se confondrait avec la particule.
fn word_forms(kana: &str, kanji: Option<&str>) -> Vec<String> {
    let kana: String = kana.chars().filter(|c| !c.is_whitespace()).collect();
    let mut forms = vec![kana.clone()];
    let conjugable = |w: &str| {
        w.chars()
            .last()
            .is_some_and(|c| "うくぐすつぬぶむるい".contains(c))
    };
    if conjugable(&kana) {
        let stem: String = kana.chars().take(kana.chars().count() - 1).collect();
        if stem.chars().count() >= 2 {
            forms.push(stem);
        }
    }
    if let Some(kanji) = kanji.map(str::trim).filter(|k| !k.is_empty()) {
        forms.push(kanji.to_string());
        if conjugable(kanji) {
            let stem: String = kanji.chars().take(kanji.chars().count() - 1).collect();
            if stem.chars().any(is_kanji) {
                forms.push(stem);
            }
        }
    }
    forms
}

// Glossaire pour le traducteur : mes mots présents dans la phrase, avec leur sens.
// "ねこ : chat ; あさ : matin / matinée". Vide si aucun mot n'est reconnu.
pub(super) fn glossary(japanese: &str, knowledge: &Knowledge) -> String {
    let compact: String = japanese.chars().filter(|c| !c.is_whitespace()).collect();
    knowledge
        .words
        .iter()
        // です (être) : sans intérêt pour le traducteur, et présent dans presque toutes les phrases.
        .filter(|w| !matches!(w.kana.trim(), "です" | "だ"))
        .filter(|w| {
            word_forms(&w.kana, w.kanji.as_deref())
                .iter()
                .any(|f| compact.contains(f.as_str()))
        })
        .map(|w| {
            let meaning = w
                .meanings
                .iter()
                .map(|m| m.trim())
                .filter(|m| !m.is_empty())
                .collect::<Vec<_>>()
                .join(" / ");
            format!("{} : {meaning}", w.kana.trim())
        })
        .collect::<Vec<_>>()
        .join(" ; ")
}

// `model` réécrit en kanji, `translator` traduit avec le glossaire de mes mots.
pub(super) async fn to_french(
    ollama: &Ollama,
    model: &str,
    translator: &str,
    japanese: &str,
    knowledge: &Knowledge,
) -> Result<String, AppError> {
    let written = with_kanji(ollama, model, japanese).await;
    ollama
        .translate_ja_to_fr(translator, &written, &glossary(japanese, knowledge))
        .await
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
    fn glossaire_des_mots_de_la_phrase() {
        let word = |kana: &str, kanji: Option<&str>, meaning: &str| crate::items::KnownWord {
            kana: kana.into(),
            kanji: kanji.map(String::from),
            meanings: meaning.split(';').map(String::from).collect(),
        };
        let knowledge = Knowledge {
            words: vec![
                word("ラ ブヤディス", None, "La Bouilladisse"),
                word("あさ", None, "matin;matinée"),
                word("たべる", Some("食べる"), "manger"),
                word("のむ", None, "boire"),
                word("です", None, "être"),
                word("ねこ", None, "chat"),
            ],
            ..Knowledge::default()
        };
        // Mot en deux morceaux, sens multiples, verbe conjugué ; です et les absents écartés.
        assert_eq!(
            glossary("ラ ブヤディス の あさ に たべます です", &knowledge),
            "ラ ブヤディス : La Bouilladisse ; あさ : matin / matinée ; たべる : manger"
        );
        // Kanji : radical 食べ reconnu ; の (particule) ne déclenche pas のむ.
        assert_eq!(glossary("私の食べ物", &knowledge), "たべる : manger");
        assert_eq!(glossary("こんにちは", &knowledge), "");
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
                &Knowledge::default(),
            )
            .await
            .unwrap();
            println!("{french}");
            assert!(!french.contains("culture"), "traduction fausse : {french}");
        });
    }
}
