// Écran de l'exercice de traduction (SPEC.md 3.4) : l'IA propose une phrase, je traduis, l'IA corrige.
// Les 10 phrases de la série sont préparées dès le lancement, en arrière-plan : la première
// s'affiche dès qu'elle est prête, les suivantes arrivent pendant que je travaille.
// Clavier : Entrée valide ma traduction, puis Entrée passe à la phrase suivante (ou au bilan).

import { FormEvent, KeyboardEvent, useEffect, useRef, useState } from "react";
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
// Phrases récentes transmises à l'IA pour éviter les répétitions (Rust en garde 5 aussi).
const PREVIOUS_SENT = 5;

// Une phrase traitée, gardée pour le bilan.
type Done = { exercise: TranslationExercise; answer: string; correction: Correction };

type Phase = "setup" | "running" | "summary";

const DIRECTIONS: { id: TranslationDirection; label: string }[] = [
  { id: "fr_jp", label: "Français vers japonais" },
  { id: "jp_fr", label: "Japonais vers français" },
];

// Indication de touche, affichée à côté des boutons.
function Key({ children }: { children: string }) {
  return (
    <kbd className="ml-2 rounded border border-current/30 px-1.5 py-0.5 text-xs opacity-70">
      {children}
    </kbd>
  );
}

export default function TranslationPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [selectedLessons, setSelectedLessons] = useState<number[]>([]); // vide = toutes
  const [direction, setDirection] = useState<TranslationDirection>("fr_jp");

  const [phase, setPhase] = useState<Phase>("setup");
  const [queue, setQueue] = useState<TranslationExercise[]>([]); // phrases déjà préparées
  const [preparing, setPreparing] = useState(false); // préparation en arrière-plan en cours
  const [index, setIndex] = useState(0); // phrase affichée (0 à 9)
  const [answer, setAnswer] = useState("");
  const [correction, setCorrection] = useState<Correction | null>(null);
  const [correcting, setCorrecting] = useState(false);
  const [done, setDone] = useState<Done[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Numéro de la série en cours : une préparation lancée pour une ancienne série s'arrête d'elle-même.
  const seriesId = useRef(0);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
    // En quittant l'écran, toute préparation en cours est abandonnée.
    return () => {
      seriesId.current += 1;
    };
  }, []);

  const current: TranslationExercise | undefined = queue[index];

  // Curseur dans le champ de réponse à chaque nouvelle phrase.
  useEffect(() => {
    if (phase === "running" && current && correction === null) input.current?.focus();
  }, [phase, current, correction]);

  function toggleLesson(id: number) {
    setSelectedLessons((list) =>
      list.includes(id) ? list.filter((l) => l !== id) : [...list, id],
    );
  }

  // Prépare les phrases une par une ; chacune s'ajoute à la file dès qu'elle est prête.
  // `already` : phrases déjà prêtes (bouton Réessayer après une erreur), conservées.
  async function prepareSeries(id: number, already: TranslationExercise[] = []) {
    const prepared = [...already];
    setPreparing(true);
    try {
      while (prepared.length < SERIES_LENGTH) {
        const previous = prepared.map((e) => e.japanese).slice(-PREVIOUS_SENT);
        const next = await api.generateTranslation(selectedLessons, direction, previous);
        if (seriesId.current !== id) return; // série abandonnée entre-temps
        prepared.push(next);
        setQueue([...prepared]);
      }
    } catch (e) {
      if (seriesId.current === id) setError(errorMessage(e));
    } finally {
      if (seriesId.current === id) setPreparing(false);
    }
  }

  function startSeries() {
    seriesId.current += 1;
    setQueue([]);
    setIndex(0);
    setAnswer("");
    setCorrection(null);
    setDone([]);
    setError(null);
    setPhase("running");
    prepareSeries(seriesId.current);
  }

  function backToSetup() {
    seriesId.current += 1; // arrête la préparation
    setPreparing(false);
    setPhase("setup");
  }

  async function submitAnswer(event?: FormEvent) {
    event?.preventDefault();
    if (!current || correcting || correction !== null || answer.trim() === "") return;
    setCorrecting(true);
    setError(null);
    try {
      const result = await api.correctTranslation(current, answer);
      setCorrection(result);
      setDone((d) => [...d, { exercise: current, answer, correction: result }]);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setCorrecting(false);
    }
  }

  function goNext() {
    if (index + 1 >= SERIES_LENGTH) {
      setPhase("summary");
      return;
    }
    setIndex(index + 1);
    setAnswer("");
    setCorrection(null);
    setError(null);
  }

  // Entrée hors du champ de réponse : passer à la suite (correction lue) ou relancer (bilan).
  useEffect(() => {
    function onKey(event: globalThis.KeyboardEvent) {
      if (event.key !== "Enter" || event.repeat || event.isComposing) return;
      if (phase === "running" && correction !== null) {
        event.preventDefault();
        goNext();
      } else if (phase === "summary") {
        event.preventDefault();
        startSeries();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // Entrée dans le champ : valide, sauf pendant la conversion des kana (clavier japonais).
  function onAnswerKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter" && event.nativeEvent.isComposing) event.preventDefault();
  }

  // --- Choix de l'exercice ---
  if (phase === "setup") {
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
          <button className="btn-primary" onClick={startSeries}>
            Commencer une série de {SERIES_LENGTH} phrases
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  // --- Bilan de fin de série ---
  if (phase === "summary") {
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
                <p className="text-stone-600">Correction : {d.correction.correction}</p>
              )}
            </li>
          ))}
        </ul>
        <div className="flex gap-2">
          <button className="btn-primary" onClick={startSeries}>
            Nouvelle série
            <Key>Entrée</Key>
          </button>
          <button className="btn" onClick={backToSetup}>
            Changer de réglages
          </button>
        </div>
      </section>
    );
  }

  // --- Exercice en cours ---
  const isLast = index + 1 >= SERIES_LENGTH;

  return (
    <section className="space-y-6">
      <div className="flex items-center justify-between gap-4">
        <h2 className="text-xl font-semibold">Traduction</h2>
        <button className="btn" onClick={backToSetup}>
          Changer de réglages
        </button>
      </div>

      {/* Progression : phrase en cours, et phrases déjà préparées (plus clair). */}
      <div className="space-y-1">
        <div className="flex h-2 gap-1">
          {Array.from({ length: SERIES_LENGTH }, (_, i) => (
            <div
              key={i}
              className={`flex-1 rounded-full ${
                i < index || (i === index && correction !== null)
                  ? "bg-red-800"
                  : i < queue.length
                    ? "bg-stone-300"
                    : "bg-stone-100"
              }`}
            />
          ))}
        </div>
        <p className="text-xs text-stone-500">
          Phrase {index + 1} / {SERIES_LENGTH}
          {preparing && ` - préparation des suivantes (${queue.length} / ${SERIES_LENGTH} prêtes)`}
        </p>
      </div>

      {current === undefined ? (
        <div className="card py-10 text-center text-stone-500">
          {preparing ? "L'IA prépare la phrase..." : "Aucune phrase disponible."}
        </div>
      ) : (
        <>
          <div className="card space-y-2 py-8 text-center">
            <p className="text-xs uppercase tracking-wide text-stone-400">
              {current.direction === "fr_jp" ? "Traduis en japonais" : "Traduis en français"}
            </p>
            <p className="text-3xl">
              {current.direction === "fr_jp" ? current.french : current.japanese}
            </p>
            {current.unknown_words.length > 0 && (
              <p className="text-sm text-amber-700">
                Hors de ta base : {current.unknown_words.join(", ")}
              </p>
            )}
          </div>

          <form onSubmit={submitAnswer} className="flex gap-2">
            <input
              ref={input}
              className="field flex-1 text-lg"
              placeholder={
                current.direction === "fr_jp" ? "Ta traduction en japonais" : "Ta traduction"
              }
              value={answer}
              disabled={correction !== null || correcting}
              onChange={(e) => setAnswer(e.target.value)}
              onKeyDown={onAnswerKeyDown}
            />
            {correction === null && (
              <button
                type="submit"
                className="btn-primary"
                disabled={correcting || answer.trim() === ""}
              >
                {correcting ? "Correction..." : "Corriger"}
                {!correcting && <Key>Entrée</Key>}
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
              <p className="text-xs text-stone-400">
                Phrase de référence :{" "}
                {current.direction === "fr_jp" ? current.japanese : current.french}
              </p>
            </div>
          )}

          {correction && (
            <button className="btn-primary w-full" onClick={goNext}>
              {isLast ? "Voir le bilan" : "Phrase suivante"}
              <Key>Entrée</Key>
            </button>
          )}
        </>
      )}

      {error && (
        <div className="error flex items-center justify-between gap-4">
          <span>{error}</span>
          {!preparing && queue.length < SERIES_LENGTH && (
            <button className="btn" onClick={() => prepareSeries(seriesId.current, queue)}>
              Réessayer
            </button>
          )}
        </div>
      )}
    </section>
  );
}
