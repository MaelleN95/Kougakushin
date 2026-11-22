// Écran de révision : session de flashcards, révélation de la réponse, note FSRS.
// Clavier : Espace ou Entrée = révéler ; 1 à 4 = noter.

import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { Lesson, ReviewCard, ReviewRating } from "../types";
import { shuffled } from "../utils";

const RATINGS: { rating: ReviewRating; label: string; key: string; className: string }[] = [
  { rating: "again", label: "À revoir", key: "1", className: "btn-danger" },
  { rating: "hard", label: "Difficile", key: "2", className: "btn" },
  { rating: "good", label: "Bien", key: "3", className: "btn" },
  { rating: "easy", label: "Facile", key: "4", className: "btn" },
];

// Nombre de notes par type, pour le bilan de fin de session.
type Tally = Record<ReviewRating, number>;
const EMPTY_TALLY: Tally = { again: 0, hard: 0, good: 0, easy: 0 };

// Question selon le sens de la carte (SPEC.md 3.3).
function Question({ card }: { card: ReviewCard }) {
  switch (card.direction) {
    case "fr_jp":
      return <p className="text-3xl">{card.meaning}</p>;
    case "jp_fr":
      return <p className="text-4xl">{card.kana}</p>;
    case "kanji_read":
      return <p className="text-5xl">{card.kanji}</p>;
  }
}

// Réponse complète : la même pour les trois sens.
function Answer({ card }: { card: ReviewCard }) {
  return (
    <div className="space-y-1 border-t border-stone-200 pt-4">
      <p className="text-2xl">
        {card.kanji && <span className="mr-3">{card.kanji}</span>}
        {card.kana}
        {card.romaji && <span className="ml-3 text-base text-stone-400">{card.romaji}</span>}
      </p>
      <p className="text-lg">{card.meaning}</p>
      {card.example_jp && (
        <p className="text-sm">
          {card.example_jp}
          {card.example_fr && <span className="text-stone-500"> - {card.example_fr}</span>}
        </p>
      )}
    </div>
  );
}

const DIRECTION_LABELS = {
  fr_jp: "Français => japonais",
  jp_fr: "Japonais => français",
  kanji_read: "Lecture du kanji",
} as const; // as const : objet en lecture seule, valeurs gardées comme littéraux

export default function ReviewPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [lessonId, setLessonId] = useState<number | undefined>(undefined);
  // null = session pas encore lancée.
  const [queue, setQueue] = useState<ReviewCard[] | null>(null);
  const [index, setIndex] = useState(0);
  const [revealed, setRevealed] = useState(false);
  const [busy, setBusy] = useState(false); // évite une double note pendant l'appel à Rust
  const [tally, setTally] = useState<Tally>(EMPTY_TALLY);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
  }, []);

  const current = queue !== null && index < queue.length ? queue[index] : null;

  async function startSession() {
    try {
      setQueue(shuffled(await api.getReviewSession(lessonId)));
      setIndex(0);
      setRevealed(false);
      setTally(EMPTY_TALLY);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  async function rate(rating: ReviewRating) {
    if (current === null || busy) return;
    setBusy(true);
    try {
      await api.reviewCard(current.card_id, rating);
      setTally((t) => ({ ...t, [rating]: t[rating] + 1 }));
      setIndex((i) => i + 1);
      setRevealed(false);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  // Raccourcis clavier, réinstallés à chaque carte pour lire l'état à jour.
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (current === null) return;
      if (!revealed && (event.key === " " || event.key === "Enter")) {
        event.preventDefault();
        setRevealed(true);
        return;
      }
      const match = RATINGS.find((r) => r.key === event.key);
      if (revealed && match) rate(match.rating);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey); // nettoyage avant la prochaine pose
  }, [current, revealed, busy]);

  // --- Avant la session ---
  if (queue === null) {
    return (
      <section className="space-y-6">
        <h2 className="text-xl font-semibold">Réviser</h2>
        <div className="card flex flex-wrap items-center gap-3">
          <select
            className="field w-auto"
            value={lessonId ?? ""}
            onChange={(e) => setLessonId(e.target.value ? Number(e.target.value) : undefined)}
          >
            <option value="">Toutes les leçons</option>
            {lessons.map((lesson) => (
              <option key={lesson.id} value={lesson.id}>
                {lesson.title}
              </option>
            ))}
          </select>
          <button className="btn-primary" onClick={startSession}>
            Commencer
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  // --- Fin de session (ou rien à réviser) ---
  if (current === null) {
    const total = Object.values(tally).reduce((sum, n) => sum + n, 0);
    return (
      <section className="space-y-6">
        <h2 className="text-xl font-semibold">Réviser</h2>
        <div className="card space-y-3">
          {total === 0 ? (
            <p>Rien à réviser pour l'instant.</p>
          ) : (
            <>
              <p className="font-medium">Session terminée : {total} carte(s).</p>
              <ul className="text-sm">
                {RATINGS.map((r) => (
                  <li key={r.rating}>
                    {r.label} : {tally[r.rating]}
                  </li>
                ))}
              </ul>
            </>
          )}
          <div className="flex gap-2">
            {/* Les cartes notées "À revoir" reviennent quelques minutes plus tard. */}
            <button className="btn-primary" onClick={startSession}>
              Relancer
            </button>
            <button className="btn" onClick={() => setQueue(null)}>
              Retour
            </button>
          </div>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  // --- Carte en cours ---
  return (
    <section className="space-y-6">
      <div className="flex items-baseline justify-between">
        <h2 className="text-xl font-semibold">Réviser</h2>
        <p className="text-sm text-stone-500">
          {index + 1} / {queue.length}
        </p>
      </div>

      <div className="card space-y-4 py-10 text-center">
        <p className="text-xs uppercase tracking-wide text-stone-400">
          {DIRECTION_LABELS[current.direction]}
          {current.is_new && " - nouvelle"}
        </p>
        <Question card={current} />
        {revealed && <Answer card={current} />}
      </div>

      {revealed ? (
        <div className="grid grid-cols-4 gap-2">
          {RATINGS.map((r) => (
            <button
              key={r.rating}
              className={r.className}
              disabled={busy}
              onClick={() => rate(r.rating)}
            >
              {r.label} <span className="text-stone-400">({r.key})</span>
            </button>
          ))}
        </div>
      ) : (
        <button className="btn-primary w-full" onClick={() => setRevealed(true)}>
          Afficher la réponse (Espace)
        </button>
      )}

      {error && <p className="error">{error}</p>}
    </section>
  );
}
