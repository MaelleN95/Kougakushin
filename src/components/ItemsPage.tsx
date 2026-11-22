// Écran des éléments : filtres, formulaire rapide et liste.

import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { Item, ItemFilter, ItemKind, Lesson } from "../types";
import ItemForm, { WORD_TYPE_LABELS } from "./ItemForm";

export default function ItemsPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [filter, setFilter] = useState<ItemFilter>({ active: true });
  const [editing, setEditing] = useState<Item | null>(null);
  const [pendingDeleteId, setPendingDeleteId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function reloadItems() {
    try {
      setItems(await api.listItems(filter));
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  useEffect(() => {
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
  }, []);

  // Relance la recherche à chaque changement de filtre.
  useEffect(() => {
    reloadItems();
  }, [filter]);

  // Modifie un seul critère ; undefined = critère retiré (absent du JSON envoyé).
  function updateFilter(patch: Partial<ItemFilter>) {
    setFilter((current) => ({ ...current, ...patch }));
  }

  async function toggleActive(item: Item) {
    try {
      await api.setItemActive(item.id, !item.active);
      await reloadItems();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  async function handleDelete(id: number) {
    if (pendingDeleteId !== id) {
      setPendingDeleteId(id);
      return;
    }
    setPendingDeleteId(null);
    try {
      await api.deleteItem(id);
      if (editing?.id === id) setEditing(null);
      await reloadItems();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  if (lessons.length === 0) {
    return <p className="text-stone-500">Crée d'abord une leçon dans l'onglet "Leçons".</p>;
  }

  // Leçon proposée par défaut : celle du filtre, sinon la plus récente (liste triée par date).
  const defaultLessonId = filter.lesson_id ?? lessons[0].id;
  const lessonTitle = (id: number) => lessons.find((l) => l.id === id)?.title ?? "?";

  return (
    <section className="space-y-6">
      <h2 className="text-xl font-semibold">Éléments</h2>

      <ItemForm
        // key change => formulaire recréé : à chaque élément édité, ou si la leçon par défaut change.
        key={editing ? `edit-${editing.id}` : `new-${defaultLessonId}`}
        lessons={lessons}
        defaultLessonId={defaultLessonId}
        editing={editing}
        onSaved={() => {
          setEditing(null);
          reloadItems();
        }}
        onCancel={() => setEditing(null)}
      />

      <div className="flex flex-wrap gap-3">
        <input
          className="field flex-1"
          placeholder="Rechercher..."
          value={filter.search ?? ""}
          onChange={(e) => updateFilter({ search: e.target.value || undefined })}
        />
        <select
          className="field w-auto"
          value={filter.lesson_id ?? ""}
          onChange={(e) =>
            updateFilter({ lesson_id: e.target.value ? Number(e.target.value) : undefined })
          }
        >
          <option value="">Toutes les leçons</option>
          {lessons.map((lesson) => (
            <option key={lesson.id} value={lesson.id}>
              {lesson.title}
            </option>
          ))}
        </select>
        <select
          className="field w-auto"
          value={filter.kind ?? ""}
          onChange={(e) =>
            updateFilter({ kind: (e.target.value || undefined) as ItemKind | undefined })
          }
        >
          <option value="">Tous les types</option>
          <option value="vocab">Vocabulaire</option>
          <option value="grammar">Grammaire</option>
        </select>
        <select
          className="field w-auto"
          value={filter.active === undefined ? "" : String(filter.active)}
          onChange={(e) =>
            updateFilter({ active: e.target.value === "" ? undefined : e.target.value === "true" })
          }
        >
          <option value="true">Actifs</option>
          <option value="false">Inactifs</option>
          <option value="">Actifs et inactifs</option>
        </select>
      </div>

      {error && <p className="error">{error}</p>}

      {items.length === 0 ? (
        <p className="text-stone-500">Aucun élément ne correspond.</p>
      ) : (
        <ul className="space-y-2">
          {items.map((item) => (
            <li
              key={item.id}
              className={`card flex items-start justify-between gap-4 ${item.active ? "" : "opacity-50"}`}
            >
              <div className="min-w-0">
                {/* Narrowing : dans chaque branche, TS connaît les champs disponibles. */}
                {item.content.kind === "vocab" ? (
                  <>
                    <p className="font-medium">
                      {item.content.kanji ?? item.content.kana}
                      {item.content.kanji && (
                        <span className="ml-2 text-stone-500">{item.content.kana}</span>
                      )}
                      {item.content.romaji && (
                        <span className="ml-2 text-sm text-stone-400">{item.content.romaji}</span>
                      )}
                    </p>
                    <p>{item.content.meaning}</p>
                    {item.content.word_type && (
                      <p className="text-sm text-stone-500">
                        {WORD_TYPE_LABELS[item.content.word_type]}
                      </p>
                    )}
                  </>
                ) : (
                  <>
                    <p className="font-medium">{item.content.title}</p>
                    <p className="whitespace-pre-line">{item.content.explanation}</p>
                  </>
                )}
                {item.example_jp && (
                  <p className="mt-1 text-sm">
                    {item.example_jp}
                    {item.example_fr && <span className="text-stone-500"> - {item.example_fr}</span>}
                  </p>
                )}
                <p className="mt-1 text-xs text-stone-400">{lessonTitle(item.lesson_id)}</p>
              </div>
              <div className="flex shrink-0 flex-col gap-1 sm:flex-row">
                <button className="btn" onClick={() => setEditing(item)}>
                  Modifier
                </button>
                <button className="btn" onClick={() => toggleActive(item)}>
                  {item.active ? "Désactiver" : "Activer"}
                </button>
                <button className="btn-danger" onClick={() => handleDelete(item.id)}>
                  {pendingDeleteId === item.id ? "Confirmer ?" : "Supprimer"}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
