// Regroupement des leçons par manuel, partagé par tous les écrans qui choisissent une leçon.

import type { Book, Lesson } from "./types";

// book null = leçons sans manuel.
export type LessonGroup = { book: Book | null; lessons: Lesson[] };

// Manuels dans l'ordre reçu (alphabétique), puis "sans manuel" à la fin.
// Les leçons gardent leur ordre (la plus récente d'abord). Les groupes vides sont omis.
export function groupLessons(lessons: Lesson[], books: Book[]): LessonGroup[] {
  const groups: LessonGroup[] = books.map((book) => ({
    book,
    lessons: lessons.filter((l) => l.book_id === book.id),
  }));
  groups.push({ book: null, lessons: lessons.filter((l) => l.book_id === null) });
  return groups.filter((g) => g.lessons.length > 0);
}

// Titre d'un groupe à l'écran.
export function groupLabel(group: LessonGroup): string {
  return group.book?.title ?? "Sans manuel";
}
