// Écran des leçons : liste, ajout, modification et suppression.

import { FormEvent, useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { Lesson } from "../types";
import { emptyToNull, todayLocal } from "../utils";

export default function LessonsPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Formulaire : editingId null = création, sinon modification de cette leçon.
  const [editingId, setEditingId] = useState<number | null>(null);
  const [title, setTitle] = useState("");
  const [date, setDate] = useState(todayLocal());
  const [notes, setNotes] = useState("");

  // Suppression en deux clics : le premier arme le bouton, le second confirme.
  const [pendingDeleteId, setPendingDeleteId] = useState<number | null>(null);

  async function reload() {
    try {
      setLessons(await api.listLessons());
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  // [] : une seule fois, à l'ouverture de l'écran.
  useEffect(() => {
    reload();
  }, []);

  function resetForm() {
    setEditingId(null);
    setTitle("");
    setDate(todayLocal());
    setNotes("");
  }

  function startEdit(lesson: Lesson) {
    setEditingId(lesson.id);
    setTitle(lesson.title);
    setDate(lesson.date);
    setNotes(lesson.notes ?? ""); // ?? : null => ""
    setError(null);
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const input = { title: title.trim(), date, notes: emptyToNull(notes) };
    try {
      if (editingId === null) {
        await api.createLesson(input);
      } else {
        await api.updateLesson(editingId, input);
      }
      resetForm();
      setError(null);
      await reload();
    } catch (e) {
      setError(errorMessage(e)); // message de validation Rust, affiché tel quel
    }
  }

  async function handleDelete(id: number) {
    if (pendingDeleteId !== id) {
      setPendingDeleteId(id);
      return;
    }
    setPendingDeleteId(null);
    try {
      await api.deleteLesson(id);
      if (editingId === id) resetForm();
      setError(null);
      await reload();
    } catch (e) {
      setError(errorMessage(e)); // ex. leçon qui contient des éléments
    }
  }

  return (
    <section className="space-y-6">
      <h2 className="text-xl font-semibold">Leçons</h2>

      <form onSubmit={handleSubmit} className="card space-y-3">
        <div className="grid gap-3 sm:grid-cols-[1fr_auto]">
          <input
            className="field"
            placeholder="Titre (ex. Marugoto L5 - la famille)"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
          <input
            className="field"
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
          />
        </div>
        <textarea
          className="field"
          rows={2}
          placeholder="Notes (facultatif)"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
        />
        <div className="flex gap-2">
          <button type="submit" className="btn-primary">
            {editingId === null ? "Ajouter la leçon" : "Enregistrer"}
          </button>
          {editingId !== null && (
            <button type="button" className="btn" onClick={resetForm}>
              Annuler
            </button>
          )}
        </div>
      </form>

      {error && <p className="error">{error}</p>}

      {lessons.length === 0 ? (
        <p className="text-stone-500">Aucune leçon pour l'instant.</p>
      ) : (
        <ul className="space-y-2">
          {lessons.map((lesson) => (
            <li key={lesson.id} className="card flex items-start justify-between gap-4">
              <div>
                <p className="font-medium">{lesson.title}</p>
                <p className="text-sm text-stone-500">{lesson.date}</p>
                {lesson.notes && <p className="mt-1 text-sm">{lesson.notes}</p>}
              </div>
              <div className="flex shrink-0 gap-2">
                <button className="btn" onClick={() => startEdit(lesson)}>
                  Modifier
                </button>
                <button className="btn-danger" onClick={() => handleDelete(lesson.id)}>
                  {pendingDeleteId === lesson.id ? "Confirmer ?" : "Supprimer"}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
