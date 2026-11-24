// Écran des leçons : manuels (création, renommage, suppression), puis leçons groupées par manuel.

import { FormEvent, useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { groupLabel, groupLessons } from "../lessonGroups";
import type { Book, Lesson } from "../types";
import { emptyToNull, todayLocal } from "../utils";

export default function LessonsPage() {
  const [books, setBooks] = useState<Book[]>([]);
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Formulaire des manuels : editingBookId null = création.
  const [editingBookId, setEditingBookId] = useState<number | null>(null);
  const [bookTitle, setBookTitle] = useState("");
  const [pendingBookDeleteId, setPendingBookDeleteId] = useState<number | null>(null);

  // Formulaire des leçons : editingId null = création, sinon modification de cette leçon.
  const [editingId, setEditingId] = useState<number | null>(null);
  const [bookId, setBookId] = useState<number | null>(null); // null = sans manuel
  const [title, setTitle] = useState("");
  const [date, setDate] = useState(todayLocal());
  const [notes, setNotes] = useState("");

  // Suppression en deux clics : le premier arme le bouton, le second confirme.
  const [pendingDeleteId, setPendingDeleteId] = useState<number | null>(null);

  async function reload() {
    try {
      // Promise.all : les deux appels partent en même temps.
      const [b, l] = await Promise.all([api.listBooks(), api.listLessons()]);
      setBooks(b);
      setLessons(l);
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  // [] : une seule fois, à l'ouverture de l'écran.
  useEffect(() => {
    reload();
  }, []);

  // --- Manuels ---

  function resetBookForm() {
    setEditingBookId(null);
    setBookTitle("");
  }

  async function handleBookSubmit(event: FormEvent) {
    event.preventDefault();
    const input = { title: bookTitle.trim() };
    try {
      if (editingBookId === null) {
        const created = await api.createBook(input);
        setBookId(created.id); // la prochaine leçon ira dans ce manuel
      } else {
        await api.updateBook(editingBookId, input);
      }
      resetBookForm();
      setError(null);
      await reload();
    } catch (e) {
      setError(errorMessage(e)); // ex. titre vide ou déjà pris
    }
  }

  async function handleBookDelete(id: number) {
    if (pendingBookDeleteId !== id) {
      setPendingBookDeleteId(id);
      return;
    }
    setPendingBookDeleteId(null);
    try {
      await api.deleteBook(id);
      if (editingBookId === id) resetBookForm();
      if (bookId === id) setBookId(null);
      setError(null);
      await reload();
    } catch (e) {
      setError(errorMessage(e)); // ex. manuel qui contient des leçons
    }
  }

  // --- Leçons ---

  // Le manuel choisi est gardé : on enchaîne les leçons d'un même livre sans le resélectionner.
  function resetForm() {
    setEditingId(null);
    setTitle("");
    setDate(todayLocal());
    setNotes("");
  }

  function startEdit(lesson: Lesson) {
    setEditingId(lesson.id);
    setBookId(lesson.book_id);
    setTitle(lesson.title);
    setDate(lesson.date);
    setNotes(lesson.notes ?? ""); // ?? : null => ""
    setError(null);
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const input = { book_id: bookId, title: title.trim(), date, notes: emptyToNull(notes) };
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

  const groups = groupLessons(lessons, books);

  return (
    <section className="space-y-6">
      <h2 className="text-xl font-semibold">Leçons</h2>

      <div className="card space-y-3">
        <h3 className="font-medium">Manuels</h3>
        {books.length > 0 && (
          <ul className="space-y-1">
            {books.map((book) => (
              <li key={book.id} className="flex items-center justify-between gap-4">
                <span>{book.title}</span>
                <div className="flex shrink-0 gap-2">
                  <button
                    className="btn"
                    onClick={() => {
                      setEditingBookId(book.id);
                      setBookTitle(book.title);
                    }}
                  >
                    Renommer
                  </button>
                  <button className="btn-danger" onClick={() => handleBookDelete(book.id)}>
                    {pendingBookDeleteId === book.id ? "Confirmer ?" : "Supprimer"}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
        <form onSubmit={handleBookSubmit} className="flex flex-wrap gap-2">
          <input
            className="field flex-1"
            placeholder="Nouveau manuel (ex. Marugoto A1)"
            value={bookTitle}
            onChange={(e) => setBookTitle(e.target.value)}
          />
          <button type="submit" className="btn-primary">
            {editingBookId === null ? "Ajouter le manuel" : "Renommer"}
          </button>
          {editingBookId !== null && (
            <button type="button" className="btn" onClick={resetBookForm}>
              Annuler
            </button>
          )}
        </form>
      </div>

      <form onSubmit={handleSubmit} className="card space-y-3">
        <div className="grid gap-3 sm:grid-cols-[auto_1fr_auto]">
          <select
            className="field"
            value={bookId ?? ""}
            onChange={(e) => setBookId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">Sans manuel</option>
            {books.map((book) => (
              <option key={book.id} value={book.id}>
                {book.title}
              </option>
            ))}
          </select>
          <input
            className="field"
            placeholder="Titre (ex. L5 - la famille)"
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
        groups.map((group) => (
          <div key={group.book?.id ?? "none"} className="space-y-2">
            <h3 className="font-medium text-stone-600">{groupLabel(group)}</h3>
            <ul className="space-y-2">
              {group.lessons.map((lesson) => (
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
          </div>
        ))
      )}
    </section>
  );
}
