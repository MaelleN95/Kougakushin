// Conjugaison des verbes à partir de la forme du dictionnaire (D40), en Rust pur, sans base.
// Règles du manuel Marugoto : verbes en -ru, en -u, irréguliers (する, くる), avec les
// exceptions ある (ない, なかった) et いく (いって, いった).

use crate::items::WordType;

// Groupe de conjugaison d'un verbe. Mêmes valeurs JSON que WordType ("ru_verb"...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum VerbType {
    #[serde(rename = "ru_verb")]
    Ru, // たべる, みる
    #[serde(rename = "u_verb")]
    U, // かう, よむ
    #[serde(rename = "irregular_verb")]
    Irregular, // する, くる et leurs composés
}

impl VerbType {
    // Type de mot => groupe de verbe ; None si le mot n'est pas un verbe.
    pub(crate) fn from_word_type(word_type: WordType) -> Option<VerbType> {
        match word_type {
            WordType::RuVerb => Some(VerbType::Ru),
            WordType::UVerb => Some(VerbType::U),
            WordType::IrregularVerb => Some(VerbType::Irregular),
            _ => None,
        }
    }

    // Valeur de la colonne item.word_type => groupe ; None pour un non-verbe ou un texte inconnu.
    pub(crate) fn from_db(value: &str) -> Option<VerbType> {
        match value {
            "ru_verb" => Some(VerbType::Ru),
            "u_verb" => Some(VerbType::U),
            "irregular_verb" => Some(VerbType::Irregular),
            _ => None,
        }
    }
}

// Formes enseignées (D40) : la forme du dictionnaire, les formes polies, て / た, ない / なかった.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VerbForm {
    Dictionary,   // たべる
    Masu,         // たべます
    Masen,        // たべません
    Mashita,      // たべました
    MasenDeshita, // たべませんでした
    Te,           // たべて
    Ta,           // たべた
    Nai,          // たべない
    Nakatta,      // たべなかった
}

impl VerbForm {
    // Toutes les formes, dans l'ordre d'affichage du tableau.
    pub(crate) const ALL: [VerbForm; 9] = [
        VerbForm::Dictionary,
        VerbForm::Masu,
        VerbForm::Masen,
        VerbForm::Mashita,
        VerbForm::MasenDeshita,
        VerbForm::Te,
        VerbForm::Ta,
        VerbForm::Nai,
        VerbForm::Nakatta,
    ];

    // Valeur stockée en base (tables conjugation_form et card).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            VerbForm::Dictionary => "dictionary",
            VerbForm::Masu => "masu",
            VerbForm::Masen => "masen",
            VerbForm::Mashita => "mashita",
            VerbForm::MasenDeshita => "masen_deshita",
            VerbForm::Te => "te",
            VerbForm::Ta => "ta",
            VerbForm::Nai => "nai",
            VerbForm::Nakatta => "nakatta",
        }
    }

    // Inverse de as_str ; texte inconnu => None.
    pub(crate) fn from_db(value: &str) -> Option<VerbForm> {
        VerbForm::ALL.into_iter().find(|f| f.as_str() == value)
    }

    // Nom de la forme dans les consignes de l'IA.
    pub(crate) fn prompt_name(self) -> &'static str {
        match self {
            VerbForm::Dictionary => "forme du dictionnaire",
            VerbForm::Masu => "ます",
            VerbForm::Masen => "ません",
            VerbForm::Mashita => "ました",
            VerbForm::MasenDeshita => "ませんでした",
            VerbForm::Te => "て",
            VerbForm::Ta => "た",
            VerbForm::Nai => "ない",
            VerbForm::Nakatta => "なかった",
        }
    }
}

// Une case du tableau de conjugaison. `kanji` : la même forme écrite avec le kanji du mot
// (食べます), None si le mot n'a pas de kanji ou si sa terminaison ne correspond pas aux kana.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ConjugatedForm {
    pub(crate) form: VerbForm,
    pub(crate) kana: String,
    pub(crate) kanji: Option<String>,
}

// Tableau complet d'un verbe : une case par forme, dans l'ordre de VerbForm::ALL.
pub(crate) type Conjugation = Vec<ConjugatedForm>;

// Une forme = retirer les `drop` derniers caractères de la forme du dictionnaire, puis ajouter
// `suffix` (たべる : retirer る, ajouter ます).
struct Ending {
    drop: usize,
    suffix: String,
}

// Radicaux d'un verbe : ce qu'on retire, puis ce qu'on ajoute avant ます (i), avant ない (a),
// et les formes て / た. Le reste se déduit (ません, ました...).
struct Stems {
    drop: usize,
    i_stem: String,
    a_stem: String,
    te: String,
    ta: String,
}

// Verbe en -u : dernière kana => (rangée i, rangée a, terminaison て, terminaison た).
// う donne わ devant ない (かう => かわない).
fn u_verb_endings(last: char) -> Option<(char, char, &'static str, &'static str)> {
    match last {
        'う' => Some(('い', 'わ', "って", "った")),
        'つ' => Some(('ち', 'た', "って", "った")),
        'る' => Some(('り', 'ら', "って", "った")),
        'む' => Some(('み', 'ま', "んで", "んだ")),
        'ぶ' => Some(('び', 'ば', "んで", "んだ")),
        'ぬ' => Some(('に', 'な', "んで", "んだ")),
        'く' => Some(('き', 'か', "いて", "いた")),
        'ぐ' => Some(('ぎ', 'が', "いで", "いだ")),
        'す' => Some(('し', 'さ', "して", "した")),
        _ => None,
    }
}

// Terminaisons possibles d'un verbe en -u (indice "ressemble à un verbe").
const U_VERB_LAST_KANA: [char; 9] = ['う', 'つ', 'る', 'む', 'ぶ', 'ぬ', 'く', 'ぐ', 'す'];

fn stems(kana: &str, verb_type: VerbType) -> Result<Stems, &'static str> {
    if kana.chars().count() < 2 {
        return Err("Le verbe est trop court : saisis sa forme du dictionnaire (ex. たべる).");
    }
    let stems = |drop: usize, i: &str, a: &str, te: &str, ta: &str| Stems {
        drop,
        i_stem: i.to_string(),
        a_stem: a.to_string(),
        te: te.to_string(),
        ta: ta.to_string(),
    };
    match verb_type {
        VerbType::Ru if kana.ends_with('る') => Ok(stems(1, "", "", "て", "た")),
        VerbType::Ru => Err("Un verbe en -ru se termine par る (ex. たべる)."),
        VerbType::Irregular if kana.ends_with("する") => Ok(stems(2, "し", "し", "して", "した")),
        VerbType::Irregular if kana.ends_with("くる") => Ok(stems(2, "き", "こ", "きて", "きた")),
        VerbType::Irregular => {
            Err("Un verbe irrégulier se termine par する ou くる (ex. べんきょうする).")
        }
        VerbType::U => {
            let last = kana.chars().last().and_then(u_verb_endings);
            let Some((i, a, te, ta)) = last else {
                return Err(
                    "Un verbe en -u se termine par う, つ, る, む, ぶ, ぬ, く, ぐ ou す (ex. かう).",
                );
            };
            // Exception : いく (et ses composés, もっていく) fait いって / いった, pas いいて.
            let (te, ta) = if kana.ends_with("いく") {
                ("って", "った")
            } else {
                (te, ta)
            };
            Ok(stems(1, &i.to_string(), &a.to_string(), te, ta))
        }
    }
}

// Terminaison d'une forme, exceptions comprises.
fn ending(kana: &str, stems: &Stems, form: VerbForm) -> Ending {
    let with = |stem: &str, tail: &str| Ending {
        drop: stems.drop,
        suffix: format!("{stem}{tail}"),
    };
    match form {
        VerbForm::Dictionary => Ending {
            drop: 0,
            suffix: String::new(),
        },
        VerbForm::Masu => with(&stems.i_stem, "ます"),
        VerbForm::Masen => with(&stems.i_stem, "ません"),
        VerbForm::Mashita => with(&stems.i_stem, "ました"),
        VerbForm::MasenDeshita => with(&stems.i_stem, "ませんでした"),
        VerbForm::Te => with(&stems.te, ""),
        VerbForm::Ta => with(&stems.ta, ""),
        // Exception : ある => ない, なかった (pas あらない).
        VerbForm::Nai if kana == "ある" => Ending {
            drop: 2,
            suffix: "ない".into(),
        },
        VerbForm::Nakatta if kana == "ある" => Ending {
            drop: 2,
            suffix: "なかった".into(),
        },
        VerbForm::Nai => with(&stems.a_stem, "ない"),
        VerbForm::Nakatta => with(&stems.a_stem, "なかった"),
    }
}

// Texte sans ses `count` derniers caractères (et non octets : un kana fait 3 octets en UTF-8).
fn without_last_chars(text: &str, count: usize) -> &str {
    if count == 0 {
        return text;
    }
    match text.char_indices().rev().nth(count - 1) {
        Some((index, _)) => &text[..index],
        None => "",
    }
}

// Derniers `count` caractères d'un texte.
fn last_chars(text: &str, count: usize) -> &str {
    let len = text.chars().count();
    match text.char_indices().nth(len.saturating_sub(count)) {
        Some((index, _)) => &text[index..],
        None => "",
    }
}

// Même terminaison appliquée au mot en kanji : 食べる => 食べ + ます.
// Possible quand les caractères retirés (る) sont aussi la fin du kanji (okurigana).
// Cas particulier : 来る, dont le kanji se lit き ou こ selon la forme (来ます, 来ない).
fn kanji_form(kanji: &str, kana: &str, ending: &Ending) -> Option<String> {
    if ending.drop == 0 {
        return Some(kanji.to_string());
    }
    let dropped = last_chars(kana, ending.drop);
    if kanji.ends_with(dropped) {
        return Some(format!(
            "{}{}",
            without_last_chars(kanji, ending.drop),
            ending.suffix
        ));
    }
    if dropped == "くる" && kanji.ends_with("来る") {
        // On garde 来, on retire る, et la lecture き / こ (1er caractère du suffixe) disparaît.
        let reading_free: String = ending.suffix.chars().skip(1).collect();
        return Some(format!("{}{reading_free}", without_last_chars(kanji, 1)));
    }
    None
}

// Tableau de conjugaison d'un verbe donné à sa forme du dictionnaire.
// Erreur (message pour l'utilisatrice) si la terminaison ne correspond pas au type choisi.
pub(crate) fn conjugate(
    kana: &str,
    kanji: Option<&str>,
    verb_type: VerbType,
) -> Result<Conjugation, &'static str> {
    let kana = kana.trim();
    let kanji = kanji.map(str::trim).filter(|k| !k.is_empty());
    let stems = stems(kana, verb_type)?;
    Ok(VerbForm::ALL
        .into_iter()
        .map(|form| {
            let ending = ending(kana, &stems, form);
            ConjugatedForm {
                form,
                kana: format!("{}{}", without_last_chars(kana, ending.drop), ending.suffix),
                kanji: kanji.and_then(|k| kanji_form(k, kana, &ending)),
            }
        })
        .collect())
}

// --- Verbes à signaler dans l'onglet Éléments (D42) ---

// Problème repéré sur un mot de vocabulaire, en JSON : "type_missing" ou "invalid_ending".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VerbIssue {
    TypeMissing,   // ressemble à un verbe, mais sans type de verbe
    InvalidEnding, // type de verbe incompatible avec la terminaison (pas de conjugaison possible)
}

// Le mot ressemble-t-il à un verbe ? Indice simple, sans IA : les kana finissent comme un verbe
// (rangée u) ET un des sens français est un infinitif (manger, finir, prendre, se lever...).
// Faux positifs possibles (きのう = "hier") : c'est une suggestion, pas une erreur.
fn looks_like_verb(kana: &str, meaning: &str) -> bool {
    let kana_like_verb = kana
        .trim()
        .chars()
        .last()
        .is_some_and(|c| U_VERB_LAST_KANA.contains(&c));
    let french_infinitive = meaning.split(';').any(|sense| {
        let sense = sense.trim().to_lowercase();
        // Pronoms devant l'infinitif : se lever, s'en aller, en vouloir, y aller.
        let mut sense = sense.as_str();
        for pronoun in ["se ", "s'", "en ", "y "] {
            sense = sense.strip_prefix(pronoun).unwrap_or(sense);
        }
        sense
            .split_whitespace()
            .next()
            .is_some_and(|w| w.ends_with("er") || w.ends_with("ir") || w.ends_with("re"))
    });
    kana_like_verb && french_infinitive
}

// Problème d'un mot de vocabulaire ; None si tout va bien (ou si ce n'est pas un verbe).
pub(super) fn verb_issue(kana: &str, meaning: &str, word_type: Option<&str>) -> Option<VerbIssue> {
    match word_type.and_then(VerbType::from_db) {
        Some(verb_type) => conjugate(kana, None, verb_type)
            .err()
            .map(|_| VerbIssue::InvalidEnding),
        // Sans type, ou "autre" : le type a peut-être été oublié. Un nom ou un adjectif déclaré
        // n'est pas signalé (choix explicite de l'utilisatrice).
        None if matches!(word_type, None | Some("other")) && looks_like_verb(kana, meaning) => {
            Some(VerbIssue::TypeMissing)
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{VerbForm, VerbIssue, VerbType, conjugate, verb_issue};

    // Formes en kana dans l'ordre de VerbForm::ALL.
    fn kana_forms(kana: &str, verb_type: VerbType) -> Vec<String> {
        conjugate(kana, None, verb_type)
            .unwrap()
            .into_iter()
            .map(|f| f.kana)
            .collect()
    }

    // Vérifie dico, ます, て, た, ない, なかった (colonnes du manuel), puis les formes dérivées.
    fn check(kana: &str, verb_type: VerbType, expected: [&str; 6]) {
        let forms = kana_forms(kana, verb_type);
        let [dico, masu, te, ta, nai, nakatta] = expected;
        assert_eq!(forms[0], dico, "{kana} : forme du dictionnaire");
        assert_eq!(forms[1], masu, "{kana} : ます");
        assert_eq!(forms[5], te, "{kana} : て");
        assert_eq!(forms[6], ta, "{kana} : た");
        assert_eq!(forms[7], nai, "{kana} : ない");
        assert_eq!(forms[8], nakatta, "{kana} : なかった");
        // ません, ました, ませんでした : radical de ます + terminaison.
        let stem = masu.strip_suffix("ます").unwrap();
        assert_eq!(forms[2], format!("{stem}ません"));
        assert_eq!(forms[3], format!("{stem}ました"));
        assert_eq!(forms[4], format!("{stem}ませんでした"));
    }

    #[test]
    fn tableau_du_manuel_complet() {
        use VerbType::*;
        check(
            "する",
            Irregular,
            ["する", "します", "して", "した", "しない", "しなかった"],
        );
        check(
            "くる",
            Irregular,
            ["くる", "きます", "きて", "きた", "こない", "こなかった"],
        );
        check(
            "たべる",
            Ru,
            [
                "たべる",
                "たべます",
                "たべて",
                "たべた",
                "たべない",
                "たべなかった",
            ],
        );
        check(
            "かう",
            U,
            [
                "かう",
                "かいます",
                "かって",
                "かった",
                "かわない",
                "かわなかった",
            ],
        );
        check(
            "まつ",
            U,
            [
                "まつ",
                "まちます",
                "まって",
                "まった",
                "またない",
                "またなかった",
            ],
        );
        check(
            "とる",
            U,
            [
                "とる",
                "とります",
                "とって",
                "とった",
                "とらない",
                "とらなかった",
            ],
        );
        check(
            "ある",
            U,
            ["ある", "あります", "あって", "あった", "ない", "なかった"],
        );
        check(
            "よむ",
            U,
            [
                "よむ",
                "よみます",
                "よんで",
                "よんだ",
                "よまない",
                "よまなかった",
            ],
        );
        check(
            "あそぶ",
            U,
            [
                "あそぶ",
                "あそびます",
                "あそんで",
                "あそんだ",
                "あそばない",
                "あそばなかった",
            ],
        );
        check(
            "しぬ",
            U,
            [
                "しぬ",
                "しにます",
                "しんで",
                "しんだ",
                "しなない",
                "しななかった",
            ],
        );
        check(
            "かく",
            U,
            [
                "かく",
                "かきます",
                "かいて",
                "かいた",
                "かかない",
                "かかなかった",
            ],
        );
        check(
            "いく",
            U,
            [
                "いく",
                "いきます",
                "いって",
                "いった",
                "いかない",
                "いかなかった",
            ],
        );
        check(
            "いそぐ",
            U,
            [
                "いそぐ",
                "いそぎます",
                "いそいで",
                "いそいだ",
                "いそがない",
                "いそがなかった",
            ],
        );
        check(
            "はなす",
            U,
            [
                "はなす",
                "はなします",
                "はなして",
                "はなした",
                "はなさない",
                "はなさなかった",
            ],
        );
    }

    #[test]
    fn verbes_composes_en_suru_et_kuru() {
        use VerbType::Irregular;
        check(
            "べんきょうする",
            Irregular,
            [
                "べんきょうする",
                "べんきょうします",
                "べんきょうして",
                "べんきょうした",
                "べんきょうしない",
                "べんきょうしなかった",
            ],
        );
        check(
            "もってくる",
            Irregular,
            [
                "もってくる",
                "もってきます",
                "もってきて",
                "もってきた",
                "もってこない",
                "もってこなかった",
            ],
        );
        // Composé de いく : même exception.
        check(
            "もっていく",
            VerbType::U,
            [
                "もっていく",
                "もっていきます",
                "もっていって",
                "もっていった",
                "もっていかない",
                "もっていかなかった",
            ],
        );
    }

    // Formes en kanji dans l'ordre de VerbForm::ALL (None => "-").
    fn kanji_forms(kana: &str, kanji: &str, verb_type: VerbType) -> Vec<String> {
        conjugate(kana, Some(kanji), verb_type)
            .unwrap()
            .into_iter()
            .map(|f| f.kanji.unwrap_or_else(|| "-".into()))
            .collect()
    }

    #[test]
    fn forme_kanji_radical_kanji_et_terminaison_en_kana() {
        assert_eq!(
            kanji_forms("たべる", "食べる", VerbType::Ru),
            [
                "食べる",
                "食べます",
                "食べません",
                "食べました",
                "食べませんでした",
                "食べて",
                "食べた",
                "食べない",
                "食べなかった"
            ]
        );
        assert_eq!(
            kanji_forms("いく", "行く", VerbType::U)[5..7],
            ["行って", "行った"]
        );
        assert_eq!(kanji_forms("よむ", "読む", VerbType::U)[5], "読んで");
        assert_eq!(
            kanji_forms("べんきょうする", "勉強する", VerbType::Irregular)[1],
            "勉強します"
        );
    }

    #[test]
    fn kuru_en_kanji_se_lit_ki_ou_ko() {
        assert_eq!(
            kanji_forms("くる", "来る", VerbType::Irregular),
            [
                "来る",
                "来ます",
                "来ません",
                "来ました",
                "来ませんでした",
                "来て",
                "来た",
                "来ない",
                "来なかった"
            ]
        );
        assert_eq!(
            kanji_forms("もってくる", "持って来る", VerbType::Irregular)[7],
            "持って来ない"
        );
    }

    #[test]
    fn kanji_absent_ou_incompatible() {
        let forms = conjugate("たべる", Some("  "), VerbType::Ru).unwrap();
        assert!(forms.iter().all(|f| f.kanji.is_none()));
        // 有る : la forme ない (ある => ない) n'a pas de kanji ; les autres si.
        let forms = kanji_forms("ある", "有る", VerbType::U);
        assert_eq!(forms[1], "有ります");
        assert_eq!(forms[7], "-");
    }

    #[test]
    fn terminaison_incompatible_avec_le_type() {
        assert!(conjugate("たべ", None, VerbType::Ru).is_err());
        assert!(conjugate("いけ", None, VerbType::U).is_err());
        assert!(conjugate("たべる", None, VerbType::Irregular).is_err());
        assert!(conjugate("る", None, VerbType::Ru).is_err());
        assert!(conjugate("", None, VerbType::U).is_err());
        // Espaces autour : ignorés.
        assert_eq!(
            conjugate(" みる ", None, VerbType::Ru).unwrap()[1].kana,
            "みます"
        );
    }

    #[test]
    fn type_de_verbe_en_json_comme_word_type() {
        for (verb_type, json) in [
            (VerbType::Ru, "\"ru_verb\""),
            (VerbType::U, "\"u_verb\""),
            (VerbType::Irregular, "\"irregular_verb\""),
        ] {
            assert_eq!(
                serde_json::to_string(&verb_type).ok().as_deref(),
                Some(json)
            );
            assert_eq!(serde_json::from_str::<VerbType>(json).ok(), Some(verb_type));
        }
        assert_eq!(
            serde_json::to_string(&VerbForm::MasenDeshita)
                .ok()
                .as_deref(),
            Some("\"masen_deshita\"")
        );
    }

    #[test]
    fn forme_as_str_et_from_db_sont_inverses() {
        for form in VerbForm::ALL {
            assert_eq!(VerbForm::from_db(form.as_str()), Some(form));
        }
        assert_eq!(VerbForm::from_db("volitive"), None);
    }

    #[test]
    fn verbes_a_signaler() {
        // Ressemble à un verbe, sans type : à préciser.
        assert_eq!(
            verb_issue("たべる", "manger", None),
            Some(VerbIssue::TypeMissing)
        );
        assert_eq!(
            verb_issue("おきる", "se lever; se réveiller", Some("other")),
            Some(VerbIssue::TypeMissing)
        );
        assert_eq!(
            verb_issue("いく", "s'en aller", None),
            Some(VerbIssue::TypeMissing)
        );
        // Nom déclaré, ou mot qui ne finit pas comme un verbe : rien.
        assert_eq!(verb_issue("たべる", "manger", Some("noun")), None);
        assert_eq!(verb_issue("ほん", "livre", None), None);
        assert_eq!(verb_issue("りんご", "pomme", None), None);
        // Verbe typé correctement : rien ; terminaison incompatible : signalé.
        assert_eq!(verb_issue("たべる", "manger", Some("ru_verb")), None);
        assert_eq!(
            verb_issue("たべ", "manger", Some("ru_verb")),
            Some(VerbIssue::InvalidEnding)
        );
    }
}
