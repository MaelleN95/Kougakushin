// Appels typés aux commandes Tauri : le seul fichier qui connaît leurs noms et leurs arguments.

import { invoke } from "@tauri-apps/api/core";
import type {
  AssistantMessage,
  Book,
  BookInput,
  ChatTurn,
  ConjugatedForm,
  ConjugationFormState,
  ConversationStart,
  ConversationSummary,
  Correction,
  ImportReport,
  Item,
  ItemFilter,
  ItemInput,
  Lesson,
  LessonInput,
  OllamaStatus,
  ParticleUsage,
  ReviewRating,
  ReviewResult,
  ReviewScope,
  ReviewSession,
  Settings,
  SummaryError,
  ThemeInfo,
  TranslationDirection,
  TranslationExercise,
  VerbForm,
  VerbType,
  VerbWarning,
} from "./types";

export const api = {
  listBooks: () => invoke<Book[]>("list_books"),
  createBook: (input: BookInput) => invoke<Book>("create_book", { input }),
  updateBook: (id: number, input: BookInput) => invoke<Book>("update_book", { id, input }),
  deleteBook: (id: number) => invoke<null>("delete_book", { id }),

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
  // Actions groupées : renvoient le nombre d'éléments concernés.
  setItemsActive: (ids: number[], active: boolean) =>
    invoke<number>("set_items_active", { ids, active }),
  moveItems: (ids: number[], lessonId: number) =>
    invoke<number>("move_items", { ids, lessonId }),
  deleteItems: (ids: number[]) => invoke<number>("delete_items", { ids }),

  // scope vide => toutes les leçons ; extraNew : nouvelles cartes au-delà de la limite.
  getReviewSession: (scope: ReviewScope, extraNew = 0) =>
    invoke<ReviewSession>("get_review_session", { ...scope, extraNew }),
  reviewCard: (cardId: number, rating: ReviewRating) =>
    invoke<ReviewResult>("review_card", { cardId, rating }),

  // Tableau de conjugaison calculé par Rust ; erreur si la terminaison ne va pas avec le type.
  conjugateVerb: (kana: string, kanji: string | null, verbType: VerbType) =>
    invoke<ConjugatedForm[]>("conjugate_verb", { kana, kanji, verbType }),
  listConjugationForms: () => invoke<ConjugationFormState[]>("list_conjugation_forms"),
  // Désactiver une forme supprime ses cartes (et leur progression).
  setConjugationFormKnown: (form: VerbForm, known: boolean) =>
    invoke<ConjugationFormState[]>("set_conjugation_form_known", { form, known }),
  listVerbWarnings: () => invoke<VerbWarning[]>("list_verb_warnings"),

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
  // errors : corrections renvoyées par reviewConversationMessage pendant la conversation.
  endConversation: (
    lessonIds: number[],
    themeId: string,
    history: ChatTurn[],
    errors: SummaryError[],
  ) => invoke<ConversationSummary>("end_conversation", { lessonIds, themeId, history, errors }),
  // Correction de mon dernier message, en arrière-plan ; null = pas d'erreur.
  reviewConversationMessage: (lessonIds: number[], themeId: string, history: ChatTurn[]) =>
    invoke<SummaryError | null>("review_conversation_message", { lessonIds, themeId, history }),
  // Kanji de mon message absents de ma base (erreur de conversion du clavier probable).
  checkMyMessage: (text: string) => invoke<string[]>("check_my_message", { text }),
  // Traduction japonais => français par le traducteur dédié.
  translateMessage: (text: string) => invoke<string>("translate_message", { text }),

  listParticleUsages: () => invoke<ParticleUsage[]>("list_particle_usages"),
  setParticleUsageActive: (id: number, active: boolean) =>
    invoke<ParticleUsage>("set_particle_usage_active", { id, active }),
  // Tous les emplois d'une particule ; renvoie le nombre d'emplois concernés.
  setParticleActive: (particle: string, active: boolean) =>
    invoke<number>("set_particle_active", { particle, active }),

  // null : fenêtre fermée sans choisir de fichier.
  exportData: () => invoke<string | null>("export_data"),
  importData: () => invoke<ImportReport | null>("import_data"),
};

// Les erreurs Rust arrivent sous forme de chaîne (voir src-tauri/src/error.rs).
// `unknown` : TS oblige à vérifier le type avant d'utiliser la valeur.
export function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : "Erreur inattendue";
}
