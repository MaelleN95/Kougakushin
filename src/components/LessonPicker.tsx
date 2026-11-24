// Cases à cocher des leçons, groupées par manuel (Traduction, Conversation).
// La case d'un manuel coche ou décoche toutes ses leçons d'un coup (D30).

import { groupLabel, type LessonGroup } from "../lessonGroups";

type Props = {
  groups: LessonGroup[];
  selected: number[]; // vide = toutes
  onChange: (selected: number[]) => void;
};

export default function LessonPicker({ groups, selected, onChange }: Props) {
  function toggleLesson(id: number) {
    onChange(selected.includes(id) ? selected.filter((l) => l !== id) : [...selected, id]);
  }

  function toggleGroup(group: LessonGroup) {
    const ids = group.lessons.map((l) => l.id);
    const allChecked = ids.every((id) => selected.includes(id));
    onChange(
      allChecked
        ? selected.filter((id) => !ids.includes(id))
        : [...selected, ...ids.filter((id) => !selected.includes(id))],
    );
  }

  return (
    <div className="space-y-2">
      <p className="text-sm font-medium">Leçons (aucune cochée = toutes)</p>
      {groups.map((group) => {
        const checkedCount = group.lessons.filter((l) => selected.includes(l.id)).length;
        return (
          <div key={group.book?.id ?? "none"} className="space-y-1">
            <label className="flex items-center gap-2 text-sm font-medium">
              <input
                type="checkbox"
                checked={checkedCount === group.lessons.length}
                // Case à moitié cochée si une partie des leçons seulement l'est.
                ref={(el) => {
                  if (el) el.indeterminate = checkedCount > 0 && checkedCount < group.lessons.length;
                }}
                onChange={() => toggleGroup(group)}
              />
              {groupLabel(group)}
            </label>
            {group.lessons.map((lesson) => (
              <label key={lesson.id} className="ml-6 flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={selected.includes(lesson.id)}
                  onChange={() => toggleLesson(lesson.id)}
                />
                {lesson.title}
              </label>
            ))}
          </div>
        );
      })}
    </div>
  );
}
