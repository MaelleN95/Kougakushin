// Écran de conversation (SPEC.md 3.5) : l'IA joue un personnage en japonais, je réponds,
// je peux demander la traduction ou un indice ; "Terminer" affiche le bilan en français.

import { FormEvent, KeyboardEvent, useEffect, useRef, useState } from "react";
import { api, errorMessage } from "../api";
import { groupLessons } from "../lessonGroups";
import LessonPicker from "./LessonPicker";
import type {
  AssistantMessage,
  Book,
  ChatTurn,
  ConversationStart,
  ConversationSummary,
  Lesson,
  SummaryError,
  ThemeInfo,
} from "../types";

// Message affiché : celui de l'IA garde ses aides (traduction, indice) et leur état d'affichage.
type Entry =
  | {
      role: "assistant";
      message: AssistantMessage;
      french: string | null; // traduction, chargée au premier clic puis gardée
      showFrench: boolean;
      showHint: boolean;
    }
  // unknownKanji : kanji hors de ma base, signalés dès l'envoi (D35).
  | { role: "user"; text: string; unknownKanji: string[] };

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
  const [books, setBooks] = useState<Book[]>([]);
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
  const input = useRef<HTMLInputElement>(null);
  // Corrections de mes messages, lancées en arrière-plan à chaque envoi (D34) ; attendues
  // au moment du bilan. Une ref : leur arrivée ne doit pas redessiner l'écran.
  const reviews = useRef<Promise<SummaryError | null>[]>([]);

  useEffect(() => {
    api.listLessons().then(setLessons).catch((e) => setError(errorMessage(e)));
    api.listBooks().then(setBooks).catch((e) => setError(errorMessage(e)));
    api.listConversationThemes().then(setThemes).catch((e) => setError(errorMessage(e)));
  }, []);

  // Garde le dernier message visible.
  useEffect(() => {
    bottom.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries, waiting]);

  // Curseur dans le champ de réponse dès que l'IA a répondu : on enchaîne au clavier.
  useEffect(() => {
    if (start !== null && summary === null && waiting === null) input.current?.focus();
  }, [start, summary, waiting]);

  function assistantEntry(message: AssistantMessage): Entry {
    return { role: "assistant", message, french: null, showFrench: false, showHint: false };
  }

  async function startConversation() {
    setWaiting("start");
    setError(null);
    try {
      const started = await api.startConversation(selectedLessons, themeId);
      setStart(started);
      setEntries([assistantEntry(started.message)]);
      reviews.current = [];
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
    const text = answer.trim();
    const mineIndex = entries.length;
    const withMine: Entry[] = [...entries, { role: "user", text, unknownKanji: [] }];
    const history = toHistory(withMine);
    setEntries(withMine);
    setAnswer("");
    setWaiting("reply");
    setError(null);

    // Kanji hors de ma base : contrôle immédiat par le code, affiché sous mon message.
    api
      .checkMyMessage(text)
      .then((unknownKanji) =>
        setEntries((list) =>
          list.map((e, i) => (i === mineIndex && e.role === "user" ? { ...e, unknownKanji } : e)),
        ),
      )
      .catch((e) => setError(errorMessage(e)));

    try {
      // On renvoie le thème tiré au départ (et pas "surprise", qui en tirerait un autre).
      const replyCall = api.replyConversation(selectedLessons, start.theme_id, history);
      // Correction lancée après la réponse, pour qu'Ollama traite la réponse en premier.
      // Échec : message non corrigé, sans bloquer la conversation.
      reviews.current.push(
        api
          .reviewConversationMessage(selectedLessons, start.theme_id, history)
          .catch(() => null),
      );
      const reply = await replyCall;
      setEntries((list) => [...list, assistantEntry(reply)]);
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
      // Attend les corrections encore en cours, puis garde les vraies erreurs.
      const errors = (await Promise.all(reviews.current)).filter(
        (e): e is SummaryError => e !== null,
      );
      setSummary(
        await api.endConversation(selectedLessons, start.theme_id, toHistory(entries), errors),
      );
    } catch (e) {
      setError(errorMessage(e)); // ex. aucune réponse de ma part
    } finally {
      setWaiting(null);
    }
  }

  // Bouton "Traduction" : au premier clic, on demande la traduction au traducteur dédié
  // (environ 1 à 2 s), puis on la garde pour les clics suivants.
  async function toggleFrench(index: number) {
    const entry = entries[index];
    if (entry.role !== "assistant") return;
    if (entry.french === null) {
      try {
        const french = await api.translateMessage(entry.message.japanese);
        setEntries((list) =>
          list.map((e, i) =>
            i === index && e.role === "assistant" ? { ...e, french, showFrench: true } : e,
          ),
        );
      } catch (e) {
        setError(errorMessage(e));
      }
      return;
    }
    toggle(index, "showFrench");
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
          <LessonPicker
            groups={groupLessons(lessons, books)}
            selected={selectedLessons}
            onChange={setSelectedLessons}
          />
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
      {start.limited_vocabulary && (
        <p className="rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800">
          Vocabulaire encore limité (moins de 30 mots actifs) : la conversation restera très
          simple, et l'IA adaptera la situation à ce que tu sais dire.
        </p>
      )}

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
              {entry.showFrench && entry.french && (
                <p className="text-sm text-stone-600">{entry.french}</p>
              )}
              {entry.showHint && (
                <p className="text-sm text-stone-600">Indice : {entry.message.hint}</p>
              )}
              <div className="flex gap-2">
                <button
                  className="btn"
                  // preventDefault : le clic ne retire pas le curseur du champ de réponse,
                  // sinon Entrée "recliquerait" ce bouton au lieu d'envoyer.
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => toggleFrench(i)}
                >
                  {entry.showFrench ? "Masquer la traduction" : "Traduction"}
                </button>
                <button
                  className="btn"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => toggle(i, "showHint")}
                >
                  {entry.showHint ? "Masquer l'indice" : "Indice"}
                </button>
              </div>
            </div>
          ) : (
            <div key={i} className="ml-12 space-y-1 rounded-lg bg-red-800 p-4 text-white">
              <p className="text-lg">{entry.text}</p>
              {entry.unknownKanji.length > 0 && (
                <p className="text-xs text-red-100">
                  Kanji hors de ta base : {entry.unknownKanji.join(", ")}. Vérifie la conversion
                  du clavier.
                </p>
              )}
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
          ref={input}
          placeholder="Ta réponse en japonais"
          value={answer}
          onChange={(e) => setAnswer(e.target.value)}
          onKeyDown={onKeyDown}
        />
        <button type="submit" className="btn-primary" disabled={waiting !== null}>
          Envoyer
          <kbd className="ml-2 rounded border border-current/30 px-1.5 py-0.5 text-xs opacity-70">
            Entrée
          </kbd>
        </button>
      </form>
    </section>
  );
}
