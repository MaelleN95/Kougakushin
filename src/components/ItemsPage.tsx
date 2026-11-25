// Écran des éléments : filtres, formulaire rapide et liste.

import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { groupLessons } from "../lessonGroups";
import type { Book, Item, ItemFilter, ItemKind, Lesson, VerbForm, VerbWarning } from "../types";
import { VerbConjugation } from "./ConjugationTable";
import ItemForm, { WORD_TYPE_LABELS } from "./ItemForm";
import LessonOptions from "./LessonOptions";
import MeaningTags from "./MeaningTags";
import SelectionBar from "./SelectionBar";
import { isVerbType } from "./VerbFields";

// Message affiché sous un mot signalé par Rust (list_verb_warnings).
const VERB_WARNING_LABELS = {
  type_missing: "Type de verbe à préciser ? (le mot ressemble à un verbe)",
  invalid_ending: "Type de verbe incompatible avec la terminaison : pas de conjugaison",
} as const;

export default function ItemsPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [books, setBooks] = useState<Book[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [filter, setFilter] = useState<ItemFilter>({ active: true });
  const [editing, setEditing] = useState<Item | null>(null);
  const [pendingDeleteId, setPendingDeleteId] = useState<number | null>(null);
  const [showRomaji, setShowRomaji] = useState(true); // réglage
  const [error, setError] = useState<string | null>(null);
  // Éléments cochés ; limités aux éléments affichés, pour ne jamais agir sur un élément caché.
  const [selected, setSelected] = useState<number[]>([]);
  const [bulkBusy, setBulkBusy] = useState(false);
  const [bulkMessage, setBulkMessage] = useState<string | null>(null);
  // Verbes : formes connues (réglage), mots signalés, tableau de conjugaison déplié.
  const [knownForms, setKnownForms] = useState<VerbForm[]>([]);
  const [verbWarnings, setVerbWarnings] = useState<VerbWarning[]>([]);
  const [openConjugationId, setOpenConjugationId] = useState<number | null>(null);

  async function reloadItems() {
    try {
      const list = await api.listItems(filter);
      setItems(list);
      setSelected((current) => current.filter((id) => list.some((item) => item.id === id)));
      setVerbWarnings(await api.listVerbWarnings());
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  useEffect(() => {
    api
      .listConjugationForms()
      .then((forms) => setKnownForms(forms.filter((f) => f.known).map((f) => f.form)))
      .catch((e) => setError(errorMessage(e)));
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
    api.listBooks().then(setBooks).catch((e) => setError(errorMessage(e)));
    api
      .getSettings()
      .then((s) => setShowRomaji(s.show_romaji))
      .catch((e) => setError(errorMessage(e)));
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

  function toggleSelected(id: number) {
    setSelected((current) =>
      current.includes(id) ? current.filter((s) => s !== id) : [...current, id],
    );
  }

  // Lance une action groupée, affiche le nombre d'éléments traités, puis recharge la liste.
  async function runBulk(action: () => Promise<number>, done: string) {
    setBulkBusy(true);
    setBulkMessage(null);
    try {
      const count = await action();
      if (editing !== null && selected.includes(editing.id)) setEditing(null);
      setBulkMessage(`${count} élément(s) ${done}.`);
      setError(null);
      await reloadItems();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBulkBusy(false);
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
  const lessonGroups = groupLessons(lessons, books);
  // Leçon affichée sous un élément, précédée de son manuel s'il en a un.
  const lessonTitle = (id: number) => {
    const lesson = lessons.find((l) => l.id === id);
    if (!lesson) return "?";
    const book = books.find((b) => b.id === lesson.book_id);
    return book ? `${book.title} - ${lesson.title}` : lesson.title;
  };

  return (
    <section className="space-y-6">
      <h2 className="text-xl font-semibold">Éléments</h2>

      <ItemForm
        // key change => formulaire recréé : à chaque élément édité, ou si la leçon par défaut change.
        key={editing ? `edit-${editing.id}` : `new-${defaultLessonId}`}
        lessonGroups={lessonGroups}
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
          <LessonOptions groups={lessonGroups} />
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

      {items.length > 0 && (
        <SelectionBar
          count={selected.length}
          visibleCount={items.length}
          lessonGroups={lessonGroups}
          busy={bulkBusy}
          onSelectAll={() => setSelected(items.map((item) => item.id))}
          onClear={() => setSelected([])}
          onSetActive={(active) =>
            runBulk(
              () => api.setItemsActive(selected, active),
              active ? "activé(s)" : "désactivé(s)",
            )
          }
          onMove={(lessonId) => runBulk(() => api.moveItems(selected, lessonId), "déplacé(s)")}
          onDelete={() => runBulk(() => api.deleteItems(selected), "supprimé(s)")}
        />
      )}
      {bulkMessage && <p className="text-sm text-stone-600">{bulkMessage}</p>}
      {error && <p className="error">{error}</p>}

      {items.length === 0 ? (
        <p className="text-stone-500">Aucun élément ne correspond.</p>
      ) : (
        <ul className="space-y-2">
          {items.map((item) => (
            <li
              key={item.id}
              // Sélectionné : bordure et fond rouges légers (couleur d'accent de l'app).
              className={`card flex items-start gap-3 ${item.active ? "" : "opacity-50"} ${
                selected.includes(item.id) ? "border-red-700 bg-red-50 ring-1 ring-red-700" : ""
              }`}
            >
              <input
                type="checkbox"
                className="mt-1.5 shrink-0"
                aria-label="Sélectionner"
                checked={selected.includes(item.id)}
                onChange={() => toggleSelected(item.id)}
              />
              <div className="min-w-0 flex-1">
                {/* Narrowing : dans chaque branche, TS connaît les champs disponibles. */}
                {item.content.kind === "vocab" ? (
                  <>
                    <p className="font-medium">
                      {item.content.kanji ?? item.content.kana}
                      {item.content.kanji && (
                        <span className="ml-2 text-stone-500">{item.content.kana}</span>
                      )}
                      {showRomaji && item.content.romaji && (
                        <span className="ml-2 text-sm text-stone-400">{item.content.romaji}</span>
                      )}
                    </p>
                    <div className="mt-1">
                      <MeaningTags meanings={item.content.meanings} />
                    </div>
                    {item.content.word_type && (
                      <p className="text-sm text-stone-500">
                        {WORD_TYPE_LABELS[item.content.word_type]}
                      </p>
                    )}
                    {verbWarnings
                      .filter((w) => w.item_id === item.id)
                      .map((w) => (
                        <p key={w.issue} className="text-sm text-amber-700">
                          {VERB_WARNING_LABELS[w.issue]}
                        </p>
                      ))}
                    {isVerbType(item.content.word_type) && (
                      <div className="mt-1">
                        <button
                          className="text-sm text-red-800 underline"
                          onClick={() =>
                            setOpenConjugationId(openConjugationId === item.id ? null : item.id)
                          }
                        >
                          {openConjugationId === item.id ? "Masquer la conjugaison" : "Conjugaison"}
                        </button>
                        {openConjugationId === item.id && (
                          <VerbConjugation
                            kana={item.content.kana}
                            kanji={item.content.kanji}
                            verbType={item.content.word_type}
                            known={knownForms}
                          />
                        )}
                      </div>
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
