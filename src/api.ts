// Appels typés aux commandes Tauri : le seul fichier qui connaît leurs noms et leurs arguments.

import { invoke } from "@tauri-apps/api/core";
import type {
  AssistantMessage,
  ChatTurn,
  ConversationStart,
  ConversationSummary,
  Correction,
  Item,
  ItemFilter,
  ItemInput,
  Lesson,
  LessonInput,
  OllamaStatus,
  ReviewRating,
  ReviewResult,
  ReviewSession,
  Settings,
  ThemeInfo,
  TranslationDirection,
  TranslationExercise,
} from "./types";

export const api = {
  listLessons: () => invoke<Lesson[]>("list_lessons"),
  createLesson: (input: LessonInput) => invoke<Lesson>("create_lesson", { input }),
  updateLesson: (id: number, input: LessonInput) =>
    invoke<Lesson>("update_lesson", { id, input }),
  deleteLesson: (id: number) => invoke<null>("delete_lesson", { id }),

  listItems: (filter: ItemFilter = {}) => invoke<Item[]>("list_items", { filter }),
  createItem: (input: ItemInput) => invoke<Item>("create_item", { input }),
  updateItem: (id: number, input: ItemInput) => invoke<Item>("update_item", { id, input }),
  setItemActive: (id: number, active: boolean) =>
    invoke<Item>("set_item_active", { id, active }),
  deleteItem: (id: number) => invoke<null>("delete_item", { id }),

  // lessonId absent => toutes les leçons ; extraNew : nouvelles cartes au-delà de la limite.
  getReviewSession: (lessonId?: number, extraNew = 0) =>
    invoke<ReviewSession>("get_review_session", { lessonId, extraNew }),
  reviewCard: (cardId: number, rating: ReviewRating) =>
    invoke<ReviewResult>("review_card", { cardId, rating }),

  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),

  checkOllama: () => invoke<OllamaStatus>("check_ollama"),
  // lessonIds vide = toutes les leçons ; previous : phrases déjà proposées (pas de répétition).
  generateTranslation: (
    lessonIds: number[],
    direction: TranslationDirection,
    previous: string[],
  ) => invoke<TranslationExercise>("generate_translation", { lessonIds, direction, previous }),
  correctTranslation: (exercise: TranslationExercise, answer: string) =>
    invoke<Correction>("correct_translation", { exercise, answer }),

  listConversationThemes: () => invoke<ThemeInfo[]>("list_conversation_themes"),
  // themeId "surprise" = tirage au sort côté Rust.
  startConversation: (lessonIds: number[], themeId: string) =>
    invoke<ConversationStart>("start_conversation", { lessonIds, themeId }),
  replyConversation: (lessonIds: number[], themeId: string, history: ChatTurn[]) =>
    invoke<AssistantMessage>("reply_conversation", { lessonIds, themeId, history }),
  endConversation: (lessonIds: number[], themeId: string, history: ChatTurn[]) =>
    invoke<ConversationSummary>("end_conversation", { lessonIds, themeId, history }),
  // Traduction japonais => français par le traducteur dédié.
  translateMessage: (text: string) => invoke<string>("translate_message", { text }),
};

// Les erreurs Rust arrivent sous forme de chaîne (voir src-tauri/src/error.rs).
// `unknown` : TS oblige à vérifier le type avant d'utiliser la valeur.
export function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : "Erreur inattendue";
}
