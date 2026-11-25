// Écran de révision : session de flashcards, révélation de la réponse, note FSRS.
// Clavier : Espace ou Entrée = révéler ; 1 à 4 = noter.

import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { groupLessons } from "../lessonGroups";
import type { Book, Lesson, ReviewCard, ReviewRating, ReviewScope } from "../types";
import { VERB_FORM_LABELS } from "./ConjugationTable";
import LessonOptions from "./LessonOptions";
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

// Nouvelles cartes ajoutées par le bouton "de plus" quand la limite du jour est à 0.
const DEFAULT_EXTRA_STEP = 10;

// Question selon le sens de la carte (SPEC.md 3.3).
function Question({ card }: { card: ReviewCard }) {
  switch (card.direction) {
    case "fr_jp":
      return <p className="text-3xl">{card.meaning}</p>;
    case "jp_fr":
      return <p className="text-4xl">{card.kana}</p>;
    case "kanji_read":
      return <p className="text-5xl">{card.kanji}</p>;
    case "conjugation":
      // "たべる - forme en て ?" ; pour la forme du dictionnaire, la question part de ます.
      return card.conjugation === null ? null : (
        <div className="space-y-2">
          <p className="text-4xl">
            {card.conjugation.question_kanji ?? card.conjugation.question_kana}
          </p>
          <p className="text-lg text-stone-600">{VERB_FORM_LABELS[card.conjugation.form]} ?</p>
        </div>
      );
  }
}

// Réponse complète : la même pour les trois premiers sens ; la forme demandée pour la conjugaison.
function Answer({ card, showRomaji }: { card: ReviewCard; showRomaji: boolean }) {
  if (card.conjugation !== null) {
    const { answer_kana, answer_kanji } = card.conjugation;
    return (
      <div className="space-y-1 border-t border-stone-200 pt-4">
        <p className="text-2xl">
          {answer_kanji !== null && answer_kanji !== answer_kana && (
            <span className="mr-3">{answer_kanji}</span>
          )}
          {answer_kana}
        </p>
        <p className="text-lg">{card.meaning}</p>
      </div>
    );
  }
  return (
    <div className="space-y-1 border-t border-stone-200 pt-4">
      <p className="text-2xl">
        {card.kanji && <span className="mr-3">{card.kanji}</span>}
        {card.kana}
        {showRomaji && card.romaji && (
          <span className="ml-3 text-base text-stone-400">{card.romaji}</span>
        )}
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
  conjugation: "Conjugaison",
} as const; // as const : objet en lecture seule, valeurs gardées comme littéraux

// Valeur du menu de leçons => portée de la session envoyée à Rust.
function toScope(choice: string): ReviewScope {
  if (choice === "") return {};
  if (choice.startsWith("book:")) return { bookId: Number(choice.slice(5)) };
  return { lessonId: Number(choice) };
}

export default function ReviewPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [books, setBooks] = useState<Book[]>([]);
  // Valeur du menu : "" = tout, "12" = une leçon, "book:3" = un manuel entier.
  const [choice, setChoice] = useState("");
  const [showRomaji, setShowRomaji] = useState(true);
  const [extraStep, setExtraStep] = useState(DEFAULT_EXTRA_STEP);
  // null = session pas encore lancée.
  const [queue, setQueue] = useState<ReviewCard[] | null>(null);
  const [index, setIndex] = useState(0);
  const [revealed, setRevealed] = useState(false);
  const [busy, setBusy] = useState(false); // évite une double note pendant l'appel à Rust
  const [tally, setTally] = useState<Tally>(EMPTY_TALLY);
  // Nouvelles cartes demandées en plus de la limite du jour, cumulées pendant la visite.
  const [extraNew, setExtraNew] = useState(0);
  const [moreNewAvailable, setMoreNewAvailable] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
    api.listBooks().then(setBooks).catch((e) => setError(errorMessage(e)));
    api
      .getSettings()
      .then((s) => {
        setShowRomaji(s.show_romaji);
        if (s.new_cards_per_day > 0) setExtraStep(s.new_cards_per_day);
      })
      .catch((e) => setError(errorMessage(e)));
  }, []);

  const current = queue !== null && index < queue.length ? queue[index] : null;

  async function startSession(extra: number) {
    try {
      const session = await api.getReviewSession(toScope(choice), extra);
      setQueue(shuffled(session.cards));
      setMoreNewAvailable(session.more_new_available);
      setExtraNew(extra);
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
      // "À revoir" : la carte revient en fin de session (FSRS la reprogramme dans quelques minutes).
      if (rating === "again") setQueue((q) => (q === null ? q : [...q, current]));
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
            value={choice}
            onChange={(e) => setChoice(e.target.value)}
          >
            <option value="">Toutes les leçons</option>
            <LessonOptions groups={groupLessons(lessons, books)} withBooks />
          </select>
          <button className="btn-primary" onClick={() => startSession(extraNew)}>
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
            <p>Aucune carte à réviser pour l'instant.</p>
          ) : (
            <>
              <p className="font-medium">Session terminée : {total} révision(s).</p>
              <ul className="text-sm">
                {RATINGS.map((r) => (
                  <li key={r.rating}>
                    {r.label} : {tally[r.rating]}
                  </li>
                ))}
              </ul>
            </>
          )}
          <div className="flex flex-wrap gap-2">
            {moreNewAvailable && (
              <button className="btn-primary" onClick={() => startSession(extraNew + extraStep)}>
                Apprendre {extraStep} nouvelles de plus
              </button>
            )}
            <button className="btn" onClick={() => startSession(extraNew)}>
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

      {/* Progression de la session (la file s'allonge quand une carte "À revoir" revient). */}
      <div className="h-2 rounded-full bg-stone-100">
        <div
          className="h-2 rounded-full bg-red-800 transition-all"
          style={{ width: `${(index / queue.length) * 100}%` }}
        />
      </div>

      <div className="card space-y-4 py-10 text-center">
        <p className="text-xs uppercase tracking-wide text-stone-400">
          {DIRECTION_LABELS[current.direction]}
          {current.is_new && " - nouvelle"}
        </p>
        <Question card={current} />
        {revealed && <Answer card={current} showRomaji={showRomaji} />}
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
