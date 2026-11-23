// Types TS miroirs des structs Rust (src-tauri/src/*/model.rs) : même forme que le JSON échangé.
// Si un champ change côté Rust, il doit changer ici aussi.

// --- Leçons ---

export type Lesson = {
  id: number;
  title: string;
  date: string; // AAAA-MM-JJ
  notes: string | null;
  created_at: string; // UTC (datetime('now') de SQLite)
};

export type LessonInput = {
  title: string;
  date: string;
  notes: string | null;
};

// --- Éléments ---

// Union de littéraux : seules ces chaînes sont acceptées (miroir de l'enum Rust WordType).
export type WordType =
  | "noun"
  | "ru_verb"
  | "u_verb"
  | "irregular_verb"
  | "i_adjective"
  | "na_adjective"
  | "other";

export type ItemKind = "vocab" | "grammar";

// Union discriminée : `kind` dit quelle forme a l'objet (miroir de l'enum Rust ItemContent).
// Après un `if (content.kind === "vocab")`, TS sait que `content.kana` existe.
export type ItemContent =
  | {
      kind: "vocab";
      kana: string;
      romaji: string | null;
      kanji: string | null;
      meaning: string;
      word_type: WordType | null;
    }
  | {
      kind: "grammar";
      title: string;
      explanation: string;
    };

export type Item = {
  id: number;
  lesson_id: number;
  active: boolean;
  content: ItemContent;
  example_jp: string | null;
  example_fr: string | null;
  created_at: string;
  updated_at: string;
};

export type ItemInput = {
  lesson_id: number;
  content: ItemContent;
  example_jp: string | null;
  example_fr: string | null;
};

// `?` : champ facultatif ; absent = pas de filtre sur ce critère.
export type ItemFilter = {
  lesson_id?: number;
  kind?: ItemKind;
  active?: boolean;
  search?: string;
};

// --- Révisions ---

export type Direction = "fr_jp" | "jp_fr" | "kanji_read";

// Carte à réviser (miroir de ReviewCard dans src-tauri/src/cards/model.rs).
export type ReviewCard = {
  card_id: number;
  direction: Direction;
  is_new: boolean;
  kana: string;
  romaji: string | null;
  kanji: string | null;
  meaning: string;
  example_jp: string | null;
  example_fr: string | null;
};

export type ReviewRating = "again" | "hard" | "good" | "easy";

export type ReviewResult = {
  next_review_at: string; // date de prochaine révision, UTC
};

// Session renvoyée par get_review_session.
export type ReviewSession = {
  cards: ReviewCard[];
  more_new_available: boolean; // d'autres nouvelles cartes existent au-delà de la limite
};

// --- Réglages ---

export type Settings = {
  new_cards_per_day: number;
  show_romaji: boolean;
  ollama_model: string;
};

// --- IA ---

// Réponse de check_ollama (bouton "Tester la connexion").
export type OllamaStatus = {
  version: string;
  models: string[]; // modèles téléchargés
  configured_model_installed: boolean;
};

// --- Exercice de traduction ---

export type TranslationDirection = "fr_jp" | "jp_fr";

// Exercice généré par l'IA ; renvoyé tel quel à Rust pour la correction.
export type TranslationExercise = {
  direction: TranslationDirection;
  japanese: string;
  french: string;
  unknown_words: string[]; // mots ou kanji hors de ta base (garde-fou)
};

export type Verdict = "correct" | "almost" | "wrong";

export type Correction = {
  verdict: Verdict;
  correction: string;
  explanation: string;
};
