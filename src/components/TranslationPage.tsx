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

// Une série compte ce nombre de phrases, puis affiche un bilan.
const SERIES_LENGTH = 10;

// Une phrase traitée, gardée pour le bilan.
type Done = { exercise: TranslationExercise; answer: string; correction: Correction };

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
  const [done, setDone] = useState<Done[]>([]); // phrases corrigées de la série en cours
  const [showSummary, setShowSummary] = useState(false);
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
      const result = await api.correctTranslation(exercise, answer);
      setCorrection(result);
      setDone((d) => [...d, { exercise, answer, correction: result }]);
    } catch (e) {
      setError(errorMessage(e)); // ex. réponse vide, refusée par Rust
    } finally {
      setWaiting(null);
    }
  }

  // Nouvelle série : on vide le bilan précédent, puis première phrase.
  function startSeries() {
    setDone([]);
    setShowSummary(false);
    nextSentence();
  }

  // Retour au choix des leçons et du sens (la série en cours est abandonnée).
  function backToSetup() {
    setExercise(null);
    setDone([]);
    setShowSummary(false);
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
          <button className="btn-primary" disabled={waiting !== null} onClick={startSeries}>
            {waiting === "generate"
              ? "L'IA prépare une phrase..."
              : `Commencer une série de ${SERIES_LENGTH} phrases`}
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  // --- Bilan de fin de série ---
  if (showSummary) {
    const count = (verdict: Verdict) => done.filter((d) => d.correction.verdict === verdict).length;
    return (
      <section className="space-y-6">
        <h2 className="text-xl font-semibold">Bilan de la série</h2>
        <div className="card flex gap-6 text-sm">
          {(Object.keys(VERDICTS) as Verdict[]).map((v) => (
            <p key={v}>
              {VERDICTS[v].label} : <span className="font-semibold">{count(v)}</span>
            </p>
          ))}
        </div>
        <ul className="space-y-2">
          {done.map((d, i) => (
            <li key={i} className="card space-y-1 text-sm">
              <p className="text-stone-500">
                {d.exercise.direction === "fr_jp" ? d.exercise.french : d.exercise.japanese}
              </p>
              <p>
                <span
                  className={`mr-2 rounded px-1.5 py-0.5 text-xs ${VERDICTS[d.correction.verdict].className}`}
                >
                  {VERDICTS[d.correction.verdict].label}
                </span>
                {d.answer}
              </p>
              {d.correction.verdict !== "correct" && (
                <p className="text-stone-600">=&gt; {d.correction.correction}</p>
              )}
            </li>
          ))}
        </ul>
        <div className="flex gap-2">
          <button className="btn-primary" disabled={waiting !== null} onClick={startSeries}>
            {waiting === "generate" ? "L'IA prépare une phrase..." : "Nouvelle série"}
          </button>
          <button className="btn" onClick={backToSetup}>
            Changer de réglages
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  // --- Exercice en cours ---
  // Numéro de la phrase affichée : celles déjà corrigées, plus la courante si elle ne l'est pas.
  const position = done.length + (correction === null ? 1 : 0);
  const seriesOver = done.length >= SERIES_LENGTH;
  const toTranslate = exercise.direction === "fr_jp" ? exercise.french : exercise.japanese;
  const reference = exercise.direction === "fr_jp" ? exercise.japanese : exercise.french;

  return (
    <section className="space-y-6">
      <div className="flex items-baseline justify-between">
        <h2 className="text-xl font-semibold">
          Traduction{" "}
          <span className="text-sm font-normal text-stone-500">
            {position} / {SERIES_LENGTH}
          </span>
        </h2>
        <button className="btn" onClick={backToSetup}>
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

      {correction &&
        (seriesOver ? (
          <button className="btn-primary w-full" onClick={() => setShowSummary(true)}>
            Voir le bilan
          </button>
        ) : (
          <button className="btn-primary w-full" disabled={waiting !== null} onClick={nextSentence}>
            {waiting === "generate" ? "L'IA prépare une phrase..." : "Phrase suivante"}
          </button>
        ))}
    </section>
  );
}
