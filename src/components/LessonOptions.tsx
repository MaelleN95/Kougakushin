// Options d'un <select> de leçons, groupées par manuel (<optgroup>).
// Valeur d'une option : l'id de la leçon ; avec withBooks, aussi "book:<id>" pour un manuel entier.

import { groupLabel, type LessonGroup } from "../lessonGroups";

type Props = {
  groups: LessonGroup[];
  withBooks?: boolean; // ajoute "Tout le manuel" en tête de chaque manuel
};

export default function LessonOptions({ groups, withBooks = false }: Props) {
  // Aucun manuel : liste simple, sans titre de groupe inutile.
  if (groups.every((g) => g.book === null)) {
    return groups.flatMap((g) => g.lessons).map((lesson) => (
      <option key={lesson.id} value={lesson.id}>
        {lesson.title}
      </option>
    ));
  }
  return groups.map((group) => (
    <optgroup key={group.book?.id ?? "none"} label={groupLabel(group)}>
      {withBooks && group.book && (
        <option value={`book:${group.book.id}`}>Tout {group.book.title}</option>
      )}
      {group.lessons.map((lesson) => (
        <option key={lesson.id} value={lesson.id}>
          {lesson.title}
        </option>
      ))}
    </optgroup>
  ));
}
