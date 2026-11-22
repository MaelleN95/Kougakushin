// Structures de données des cartes et règle de génération.

use crate::items::ItemContent;

// Sens d'une carte (SPEC.md 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
    FrJp,      // français => japonais
    JpFr,      // japonais (kana) => français
    KanjiRead, // kanji => lecture + sens
}

impl Direction {
    // Tous les sens possibles (pour supprimer ceux qui ne sont plus voulus).
    pub(super) const ALL: [Direction; 3] = [Direction::FrJp, Direction::JpFr, Direction::KanjiRead];

    // Valeur de la colonne `direction` (doit correspondre au CHECK de la migration).
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Direction::FrJp => "fr_jp",
            Direction::JpFr => "jp_fr",
            Direction::KanjiRead => "kanji_read",
        }
    }
}

// Cartes qu'un élément doit avoir : 2 par mot, 3 s'il a un kanji, aucune pour la grammaire.
// Même règle que le rattrapage SQL de la migration 20261122100000.
pub(super) fn directions_for(content: &ItemContent) -> Vec<Direction> {
    match content {
        ItemContent::Vocab { kanji, .. } => {
            let mut directions = vec![Direction::FrJp, Direction::JpFr];
            // is_some_and : Some avec une condition vraie ; un kanji vide ne compte pas.
            if kanji.as_deref().is_some_and(|k| !k.trim().is_empty()) {
                directions.push(Direction::KanjiRead);
            }
            directions
        }
        ItemContent::Grammar { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, directions_for};
    use crate::items::ItemContent;

    fn vocab(kanji: Option<&str>) -> ItemContent {
        ItemContent::Vocab {
            kana: "たべる".into(),
            romaji: None,
            kanji: kanji.map(String::from),
            meaning: "manger".into(),
            word_type: None,
        }
    }

    #[test]
    fn regle_de_generation() {
        use Direction::*;
        assert_eq!(directions_for(&vocab(None)), vec![FrJp, JpFr]);
        assert_eq!(directions_for(&vocab(Some("  "))), vec![FrJp, JpFr]);
        assert_eq!(
            directions_for(&vocab(Some("食べる"))),
            vec![FrJp, JpFr, KanjiRead]
        );
        let grammar = ItemContent::Grammar {
            title: "t".into(),
            explanation: "e".into(),
        };
        assert!(directions_for(&grammar).is_empty());
    }
}
