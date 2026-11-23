// Écran de l'exercice de traduction (SPEC.md 3.4) : l'IA propose une phrase, je traduis, l'IA corrige.

import { FormEvent, KeyboardEvent, useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type {
  Correction,
  Lesson,
  TranslationDirection,
  TranslationExercise,
  Verdict,
} from "../types";

const VERDICTS: Record<Verdict, { label: string; className: string }> = {
  correct: { label: "Correct", className: "bg-green-50 text-green-800" },
  almost: { label: "Presque", className: "bg-amber-50 text-amber-800" },
  wrong: { label: "Faux", className: "bg-red-50 text-red-800" },
};

const DIRECTIONS: { id: TranslationDirection; label: string }[] = [
  { id: "fr_jp", label: "Français => japonais" },
  { id: "jp_fr", label: "Japonais => français" },
];

export default function TranslationPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [selectedLessons, setSelectedLessons] = useState<number[]>([]); // vide = toutes
  const [direction, setDirection] = useState<TranslationDirection>("fr_jp");

  const [exercise, setExercise] = useState<TranslationExercise | null>(null);
  const [answer, setAnswer] = useState("");
  const [correction, setCorrection] = useState<Correction | null>(null);
  const [previous, setPrevious] = useState<string[]>([]); // phrases déjà proposées
  // Ce que l'IA est en train de faire, pour afficher l'attente (quelques secondes).
  const [waiting, setWaiting] = useState<"generate" | "correct" | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
  }, []);

  function toggleLesson(id: number) {
    setSelectedLessons((current) =>
      current.includes(id) ? current.filter((l) => l !== id) : [...current, id],
    );
  }

  async function nextSentence() {
    setWaiting("generate");
    setError(null);
    try {
      const next = await api.generateTranslation(selectedLessons, direction, previous);
      setExercise(next);
      setPrevious((p) => [...p, next.japanese].slice(-5)); // Rust n'en garde que 5 aussi
      setAnswer("");
      setCorrection(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setWaiting(null);
    }
  }

  async function submitAnswer(event?: FormEvent) {
    event?.preventDefault();
    if (exercise === null || waiting !== null) return;
    setWaiting("correct");
    setError(null);
    try {
      setCorrection(await api.correctTranslation(exercise, answer));
    } catch (e) {
      setError(errorMessage(e)); // ex. réponse vide, refusée par Rust
    } finally {
      setWaiting(null);
    }
  }

  // Entrée valide la réponse, sauf pendant une saisie japonaise (IME) :
  // là, Entrée sert à confirmer la conversion des kana, pas à envoyer.
  function onAnswerKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter" && event.nativeEvent.isComposing) {
      event.preventDefault();
    }
  }

  // --- Choix de l'exercice ---
  if (exercise === null) {
    return (
      <section className="space-y-6">
        <h2 className="text-xl font-semibold">Traduction</h2>
        <div className="card space-y-4">
          <div className="flex gap-1">
            {DIRECTIONS.map((d) => (
              <button
                key={d.id}
                className={direction === d.id ? "btn-primary" : "btn"}
                onClick={() => setDirection(d.id)}
              >
                {d.label}
              </button>
            ))}
          </div>
          <div className="space-y-1">
            <p className="text-sm font-medium">Leçons (aucune cochée = toutes)</p>
            {lessons.map((lesson) => (
              <label key={lesson.id} className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={selectedLessons.includes(lesson.id)}
                  onChange={() => toggleLesson(lesson.id)}
                />
                {lesson.title}
              </label>
            ))}
          </div>
          <button className="btn-primary" disabled={waiting !== null} onClick={nextSentence}>
            {waiting === "generate" ? "L'IA prépare une phrase..." : "Commencer"}
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  // --- Exercice en cours ---
  const toTranslate = exercise.direction === "fr_jp" ? exercise.french : exercise.japanese;
  const reference = exercise.direction === "fr_jp" ? exercise.japanese : exercise.french;

  return (
    <section className="space-y-6">
      <div className="flex items-baseline justify-between">
        <h2 className="text-xl font-semibold">Traduction</h2>
        <button className="btn" onClick={() => setExercise(null)}>
          Changer de réglages
        </button>
      </div>

      <div className="card space-y-2 py-8 text-center">
        <p className="text-xs uppercase tracking-wide text-stone-400">
          {exercise.direction === "fr_jp" ? "Traduis en japonais" : "Traduis en français"}
        </p>
        <p className="text-3xl">{toTranslate}</p>
        {exercise.unknown_words.length > 0 && (
          <p className="text-sm text-amber-700">
            Hors de ta base : {exercise.unknown_words.join(", ")}
          </p>
        )}
      </div>

      <form onSubmit={submitAnswer} className="flex gap-2">
        <input
          className="field flex-1 text-lg"
          placeholder={exercise.direction === "fr_jp" ? "Ta traduction en japonais" : "Ta traduction"}
          value={answer}
          disabled={correction !== null}
          onChange={(e) => setAnswer(e.target.value)}
          onKeyDown={onAnswerKeyDown}
          autoFocus
        />
        {correction === null && (
          <button type="submit" className="btn-primary" disabled={waiting !== null}>
            {waiting === "correct" ? "Correction..." : "Corriger"}
          </button>
        )}
      </form>

      {correction && (
        <div className="card space-y-2">
          <p
            className={`inline-block rounded-md px-2 py-0.5 text-sm font-medium ${VERDICTS[correction.verdict].className}`}
          >
            {VERDICTS[correction.verdict].label}
          </p>
          <p className="text-lg">{correction.correction}</p>
          <p className="text-sm">{correction.explanation}</p>
          <p className="text-xs text-stone-400">Phrase de référence : {reference}</p>
        </div>
      )}

      {error && <p className="error">{error}</p>}

      {correction && (
        <button className="btn-primary w-full" disabled={waiting !== null} onClick={nextSentence}>
          {waiting === "generate" ? "L'IA prépare une phrase..." : "Phrase suivante"}
        </button>
      )}
    </section>
  );
}
