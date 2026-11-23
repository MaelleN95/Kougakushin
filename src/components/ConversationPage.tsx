// Écran de conversation (SPEC.md 3.5) : l'IA joue un personnage en japonais, je réponds,
// je peux demander la traduction ou un indice ; "Terminer" affiche le bilan en français.

import { FormEvent, KeyboardEvent, useEffect, useRef, useState } from "react";
import { api, errorMessage } from "../api";
import type {
  AssistantMessage,
  ChatTurn,
  ConversationStart,
  ConversationSummary,
  Lesson,
  ThemeInfo,
} from "../types";

// Message affiché : celui de l'IA garde ses aides (traduction, indice) et leur état d'affichage.
type Entry =
  | { role: "assistant"; message: AssistantMessage; showFrench: boolean; showHint: boolean }
  | { role: "user"; text: string };

// Historique pour Rust : seul le japonais de l'IA est renvoyé (pas ses aides).
function toHistory(entries: Entry[]): ChatTurn[] {
  return entries.map((e) =>
    e.role === "assistant"
      ? { role: "assistant", content: e.message.japanese }
      : { role: "user", content: e.text },
  );
}

export default function ConversationPage() {
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [themes, setThemes] = useState<ThemeInfo[]>([]);
  const [selectedLessons, setSelectedLessons] = useState<number[]>([]); // vide = toutes
  const [themeId, setThemeId] = useState("surprise");

  const [start, setStart] = useState<ConversationStart | null>(null); // null = pas commencée
  const [entries, setEntries] = useState<Entry[]>([]);
  const [answer, setAnswer] = useState("");
  const [summary, setSummary] = useState<ConversationSummary | null>(null);
  const [waiting, setWaiting] = useState<"start" | "reply" | "end" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const bottom = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
    api.listConversationThemes().then(setThemes).catch((e) => setError(errorMessage(e)));
  }, []);

  // Garde le dernier message visible.
  useEffect(() => {
    bottom.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries, waiting]);

  function toggleLesson(id: number) {
    setSelectedLessons((current) =>
      current.includes(id) ? current.filter((l) => l !== id) : [...current, id],
    );
  }

  function assistantEntry(message: AssistantMessage): Entry {
    return { role: "assistant", message, showFrench: false, showHint: false };
  }

  async function startConversation() {
    setWaiting("start");
    setError(null);
    try {
      const started = await api.startConversation(selectedLessons, themeId);
      setStart(started);
      setEntries([assistantEntry(started.message)]);
      setSummary(null);
      setAnswer("");
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setWaiting(null);
    }
  }

  async function send(event?: FormEvent) {
    event?.preventDefault();
    if (start === null || waiting !== null || answer.trim() === "") return;
    const withMine: Entry[] = [...entries, { role: "user", text: answer.trim() }];
    setEntries(withMine);
    setAnswer("");
    setWaiting("reply");
    setError(null);
    try {
      // On renvoie le thème tiré au départ (et pas "surprise", qui en tirerait un autre).
      const reply = await api.replyConversation(selectedLessons, start.theme_id, toHistory(withMine));
      setEntries([...withMine, assistantEntry(reply)]);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setWaiting(null);
    }
  }

  async function finish() {
    if (start === null || waiting !== null) return;
    setWaiting("end");
    setError(null);
    try {
      setSummary(await api.endConversation(selectedLessons, start.theme_id, toHistory(entries)));
    } catch (e) {
      setError(errorMessage(e)); // ex. aucune réponse de ma part
    } finally {
      setWaiting(null);
    }
  }

  // Affiche ou masque une aide d'un message de l'IA.
  function toggle(index: number, field: "showFrench" | "showHint") {
    setEntries((list) =>
      list.map((e, i) => (i === index && e.role === "assistant" ? { ...e, [field]: !e[field] } : e)),
    );
  }

  // Entrée envoie, sauf pendant la conversion des kana (clavier japonais).
  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter" && event.nativeEvent.isComposing) event.preventDefault();
  }

  // --- Choix de la conversation ---
  if (start === null) {
    return (
      <section className="space-y-6">
        <h2 className="text-xl font-semibold">Conversation</h2>
        <div className="card space-y-4">
          <div className="space-y-1">
            <p className="text-sm font-medium">Situation</p>
            <div className="flex flex-wrap gap-1">
              {[{ id: "surprise", label: "Surprise" }, ...themes].map((t) => (
                <button
                  key={t.id}
                  className={themeId === t.id ? "btn-primary" : "btn"}
                  onClick={() => setThemeId(t.id)}
                >
                  {t.label}
                </button>
              ))}
            </div>
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
          <button className="btn-primary" disabled={waiting !== null} onClick={startConversation}>
            {waiting === "start" ? "L'IA prépare la situation..." : "Commencer"}
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  // --- Bilan ---
  if (summary !== null) {
    return (
      <section className="space-y-6">
        <h2 className="text-xl font-semibold">Bilan : {start.theme_label}</h2>
        <div className="card space-y-2">
          <h3 className="font-medium">Erreurs corrigées</h3>
          {summary.errors.length === 0 ? (
            <p className="text-sm text-stone-500">Aucune erreur relevée.</p>
          ) : (
            <ul className="space-y-2 text-sm">
              {summary.errors.map((err, i) => (
                <li key={i}>
                  <p className="text-red-700 line-through">{err.student}</p>
                  <p className="text-lg">{err.correction}</p>
                  <p className="text-stone-600">{err.explanation}</p>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="card space-y-1 text-sm">
          <h3 className="font-medium">Ce qui était bien</h3>
          <ul className="list-disc pl-5">
            {summary.strengths.map((s, i) => (
              <li key={i}>{s}</li>
            ))}
          </ul>
        </div>
        <div className="card space-y-1 text-sm">
          <h3 className="font-medium">À revoir</h3>
          <ul className="list-disc pl-5">
            {summary.to_review.map((s, i) => (
              <li key={i}>{s}</li>
            ))}
          </ul>
        </div>
        <button className="btn-primary" onClick={() => setStart(null)}>
          Nouvelle conversation
        </button>
      </section>
    );
  }

  // --- Conversation en cours ---
  return (
    <section className="space-y-4">
      <div className="flex items-baseline justify-between">
        <h2 className="text-xl font-semibold">{start.theme_label}</h2>
        <button className="btn" disabled={waiting !== null} onClick={finish}>
          {waiting === "end" ? "Bilan en cours..." : "Terminer"}
        </button>
      </div>
      <p className="text-sm text-stone-500">{start.scenario}</p>

      <div className="space-y-3">
        {entries.map((entry, i) =>
          entry.role === "assistant" ? (
            <div key={i} className="card mr-12 space-y-2">
              <p className="text-xl">{entry.message.japanese}</p>
              {entry.message.unknown_words.length > 0 && (
                <p className="text-xs text-amber-700">
                  Hors de ta base : {entry.message.unknown_words.join(", ")}
                </p>
              )}
              {entry.showFrench && <p className="text-sm text-stone-600">{entry.message.french}</p>}
              {entry.showHint && (
                <p className="text-sm text-stone-600">Indice : {entry.message.hint}</p>
              )}
              <div className="flex gap-2">
                <button className="btn" onClick={() => toggle(i, "showFrench")}>
                  {entry.showFrench ? "Masquer la traduction" : "Traduction"}
                </button>
                <button className="btn" onClick={() => toggle(i, "showHint")}>
                  {entry.showHint ? "Masquer l'indice" : "Indice"}
                </button>
              </div>
            </div>
          ) : (
            <div key={i} className="ml-12 rounded-lg bg-red-800 p-4 text-lg text-white">
              {entry.text}
            </div>
          ),
        )}
        {waiting === "reply" && <p className="text-sm text-stone-500">L'IA répond...</p>}
        <div ref={bottom} />
      </div>

      {error && <p className="error">{error}</p>}

      <form onSubmit={send} className="flex gap-2">
        <input
          className="field flex-1 text-lg"
          placeholder="Ta réponse en japonais"
          value={answer}
          onChange={(e) => setAnswer(e.target.value)}
          onKeyDown={onKeyDown}
          autoFocus
        />
        <button type="submit" className="btn-primary" disabled={waiting !== null}>
          Envoyer
        </button>
      </form>
    </section>
  );
}
