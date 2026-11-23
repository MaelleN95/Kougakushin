// Briques communes aux exercices d'IA (traduction, conversation) :
// consignes construites depuis mes connaissances, et garde-fou contre les mots inconnus.

use std::collections::HashSet;

use crate::items::Knowledge;

// Longueur maximale d'une explication de grammaire dans les consignes (prompt plus court).
const MAX_EXPLANATION_CHARS: usize = 200;

// Mots grammaticaux jamais signalés comme inconnus (particules, copule, terminaisons polies).
const FUNCTION_WORDS: [&str; 18] = [
    "は",
    "が",
    "を",
    "に",
    "へ",
    "で",
    "と",
    "も",
    "の",
    "か",
    "よ",
    "ね",
    "から",
    "まで",
    "です",
    "だ",
    "ます",
    "ません",
];

// Idéogramme CJK (kanji) : blocs Unicode principal et extension A.
pub(super) fn is_kanji(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c)
}

// Katakana (bloc Unicode 30A0-30FF, avec la barre d'allongement ー), hors point médian ・.
fn is_katakana(c: char) -> bool {
    ('\u{30A1}'..='\u{30FA}').contains(&c) || c == 'ー'
}

// Mots et kanji d'une phrase absents de ma base.
// - `declared` : mots que l'IA déclare avoir utilisés (forme du dictionnaire) ;
// - kanji : vérifiés caractère par caractère dans la phrase elle-même (fiable, sans l'IA).
pub(super) fn unknown_words(
    japanese: &str,
    declared: &[String],
    knowledge: &Knowledge,
) -> Vec<String> {
    let mut known: HashSet<&str> = FUNCTION_WORDS.into_iter().collect();
    let mut known_kanji: HashSet<char> = HashSet::new();
    for word in &knowledge.words {
        known.insert(word.kana.trim());
        if let Some(kanji) = &word.kanji {
            known.insert(kanji.trim());
            known_kanji.extend(kanji.chars().filter(|&c| is_kanji(c)));
        }
    }

    let mut unknown: Vec<String> = Vec::new();
    for word in declared {
        let word = word.trim();
        if !word.is_empty() && !known.contains(word) && !unknown.iter().any(|u| u == word) {
            unknown.push(word.to_string());
        }
    }
    // Mots en katakana (emprunts : コーヒー, テーブル...) : ils ne se conjuguent pas, on peut donc
    // les comparer tels quels, même si l'IA ne les a pas déclarés. Possible grâce aux espaces (D10).
    let known_katakana = |token: &str| known.contains(token);
    for token in japanese.split_whitespace() {
        let token = token.trim_matches(|c: char| !is_katakana(c));
        if !token.is_empty()
            && token.chars().all(is_katakana)
            && !known_katakana(token)
            && !unknown.iter().any(|u| u == token)
        {
            unknown.push(token.to_string());
        }
    }
    for c in japanese.chars().filter(|&c| is_kanji(c)) {
        let kanji = c.to_string();
        if !known_kanji.contains(&c) && !unknown.contains(&kanji) {
            unknown.push(kanji);
        }
    }
    unknown
}

// Forme comparable d'une phrase japonaise, pour savoir si deux phrases sont "les mêmes" :
// sans espaces ni ponctuation, et avec mes mots en kanji réécrits en kana (食べます => たべます).
// Sert à écarter les fausses erreurs de l'IA ("correction" identique à ma phrase).
pub(super) fn normalize_japanese(text: &str, knowledge: &Knowledge) -> String {
    let mut normalized: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && !"。、．，！？!?.,「」『』・〜~".contains(*c))
        .collect();

    // Pour chaque mot connu avec kanji : partie commune finale (okurigana) retirée,
    // il reste la paire radical en kanji / radical en kana (食 / た pour 食べる / たべる).
    let mut stems: Vec<(String, String)> = Vec::new();
    for word in &knowledge.words {
        let Some(kanji) = &word.kanji else { continue };
        let kanji: Vec<char> = kanji.trim().chars().collect();
        let kana: Vec<char> = word.kana.trim().chars().collect();
        let common = kanji
            .iter()
            .rev()
            .zip(kana.iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let kanji_stem: String = kanji[..kanji.len() - common].iter().collect();
        let kana_stem: String = kana[..kana.len() - common].iter().collect();
        if kanji_stem.chars().any(is_kanji) && !kana_stem.is_empty() {
            stems.push((kanji_stem, kana_stem));
        }
    }
    // Radicaux les plus longs d'abord (学校 avant 学).
    stems.sort_by_key(|(kanji, _)| std::cmp::Reverse(kanji.chars().count()));
    for (kanji, kana) in stems {
        normalized = normalized.replace(&kanji, &kana);
    }
    normalized
}

pub(super) fn truncate(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((byte_index, _)) => format!("{}...", &text[..byte_index]),
        None => text.to_string(),
    }
}

// Règle d'écriture du japonais produit par l'IA (décisions D9 et D10).
pub(super) fn writing_rule(allowed_kanji: &str) -> String {
    let kanji = if allowed_kanji.is_empty() {
        "N'utilise AUCUN kanji.".to_string()
    } else {
        format!("Kanji autorisés UNIQUEMENT parmi : {allowed_kanji} (aucun autre kanji).")
    };
    format!(
        "Écriture : hiragana et katakana. {kanji} \
         Mets une espace entre chaque mot, et entre un mot et sa particule \
         (exemple : わたし は がっこう に いきます)."
    )
}

// Kanji présents dans mes fiches, sans doublon, dans l'ordre de saisie.
pub(super) fn allowed_kanji(knowledge: &Knowledge) -> String {
    let mut seen: Vec<char> = Vec::new();
    for word in &knowledge.words {
        for c in word
            .kanji
            .iter()
            .flat_map(|k| k.chars())
            .filter(|&c| is_kanji(c))
        {
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
    }
    seen.into_iter().collect()
}

// Règles strictes + listes de vocabulaire et de grammaire, communes à tous les exercices.
pub(super) fn knowledge_rules(knowledge: &Knowledge) -> String {
    let vocabulary: Vec<String> = knowledge
        .words
        .iter()
        .map(|w| match &w.kanji {
            Some(kanji) => format!("- {} ({kanji}) : {}", w.kana, w.meaning),
            None => format!("- {} : {}", w.kana, w.meaning),
        })
        .collect();
    let grammar: Vec<String> = knowledge
        .grammar
        .iter()
        .map(|g| {
            format!(
                "- {} : {}",
                g.title,
                truncate(&g.explanation, MAX_EXPLANATION_CHARS)
            )
        })
        .collect();
    let grammar = if grammar.is_empty() {
        "(aucun point saisi)".to_string()
    } else {
        grammar.join("\n")
    };

    format!(
        "Règles strictes :\n\
         1. Vocabulaire : UNIQUEMENT les mots de la liste ci-dessous (conjugués si besoin), \
         plus les particules.\n\
         2. Grammaire : UNIQUEMENT les points de la liste ci-dessous, plus les particules de base \
         et la forme polie です / ます (décision D11).\n\
         3. {} Écris avec leur kanji les mots de la liste qui en ont un.\n\
         4. Dans `words`, liste les noms, verbes, adjectifs et adverbes de ton japonais, à leur \
         forme du dictionnaire, écrits exactement comme dans la liste de vocabulaire \
         (sans les particules ni です / ます).\n\n\
         Vocabulaire :\n{}\n\nGrammaire :\n{grammar}",
        writing_rule(&allowed_kanji(knowledge)),
        vocabulary.join("\n"),
    )
}

// Rappel des kanji mot par mot : une consigne générale sur les kanji est ignorée par le modèle.
pub(super) fn kanji_reminder(knowledge: &Knowledge) -> Option<String> {
    let with_kanji: Vec<String> = knowledge
        .words
        .iter()
        .filter_map(|w| w.kanji.as_ref().map(|k| format!("{k} (pas {})", w.kana)))
        .collect();
    if with_kanji.is_empty() {
        None
    } else {
        Some(format!(
            "Écris obligatoirement : {}.",
            with_kanji.join(", ")
        ))
    }
}

// Tirage au sort d'un indice dans 0..len, sans dépendance : les nanosecondes varient à chaque appel.
// Suffisant pour varier des consignes ; pas pour de la sécurité.
pub(super) fn pick_index(len: usize) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    if len == 0 { 0 } else { nanos % len }
}

// Modèle des essais réels (tests ignorés) : variable KOUGAKUSHIN_MODEL, sinon le modèle par défaut.
#[cfg(test)]
pub(super) fn test_model() -> String {
    std::env::var("KOUGAKUSHIN_MODEL").unwrap_or_else(|_| "gemma4:e4b-it-q4_K_M".to_string())
}

#[cfg(test)]
mod tests {
    use super::{normalize_japanese, unknown_words};
    use crate::items::{Knowledge, KnownWord};

    fn word(kana: &str, kanji: Option<&str>) -> KnownWord {
        KnownWord {
            kana: kana.into(),
            kanji: kanji.map(String::from),
            meaning: "x".into(),
        }
    }

    #[test]
    fn katakana_inconnu_detecte_meme_non_declare() {
        let knowledge = Knowledge {
            words: vec![word("コーヒー", None)],
            ..Knowledge::default()
        };
        // テーブル n'est pas déclaré par l'IA, mais repéré ; コーヒー est connu ; ponctuation ignorée.
        let unknown = unknown_words(
            "コーヒー を どうぞ。 テーブル は どこ ですか？",
            &[],
            &knowledge,
        );
        assert_eq!(unknown, ["テーブル"]);
    }

    #[test]
    fn normalisation_ignore_espaces_ponctuation_et_kanji_connus() {
        let knowledge = Knowledge {
            words: vec![
                word("たべる", Some("食べる")),
                word("がっこう", Some("学校")),
                word("いく", Some("行く")),
            ],
            ..Knowledge::default()
        };
        let n = |t: &str| normalize_japanese(t, &knowledge);
        assert_eq!(n("すしを食べます。"), n("すし を たべます"));
        assert_eq!(n("学校 に 行きます！"), "がっこうにいきます");
        assert_ne!(n("すし を たべます"), n("すし が たべます"));
    }
}
