// Formulaire d'ajout rapide (Entrée = ajouter et rester) et de modification d'un élément.

import { FormEvent, useRef, useState } from "react";
import { toRomaji } from "wanakana";
import { api, errorMessage } from "../api";
import type { LessonGroup } from "../lessonGroups";
import type { Item, ItemContent, ItemInput, ItemKind, WordType } from "../types";
import { emptyToNull } from "../utils";
import LessonOptions from "./LessonOptions";

// Record<K, V> : un objet qui a exactement une valeur V pour chaque clé K.
// Si on ajoute un WordType sans libellé, tsc refuse de compiler.
export const WORD_TYPE_LABELS: Record<WordType, string> = {
  noun: "Nom",
  ru_verb: "Verbe en -ru",
  u_verb: "Verbe en -u",
  irregular_verb: "Verbe irrégulier",
  i_adjective: "Adjectif en -i",
  na_adjective: "Adjectif en -na",
  other: "Autre",
};

type Props = {
  lessonGroups: LessonGroup[];
  defaultLessonId: number;
  editing: Item | null; // null = création
  onSaved: () => void;
  onCancel: () => void;
};

export default function ItemForm({
  lessonGroups,
  defaultLessonId,
  editing,
  onSaved,
  onCancel,
}: Props) {
  // Valeurs de départ : celles de l'élément modifié, sinon vides.
  // Le parent change la `key` du formulaire à chaque élément édité, ce qui le recrée.
  const vocab = editing?.content.kind === "vocab" ? editing.content : null;
  const grammar = editing?.content.kind === "grammar" ? editing.content : null;

  const [lessonId, setLessonId] = useState(editing?.lesson_id ?? defaultLessonId);
  const [kind, setKind] = useState<ItemKind>(editing?.content.kind ?? "vocab");
  const [kana, setKana] = useState(vocab?.kana ?? "");
  const [romaji, setRomaji] = useState(vocab?.romaji ?? "");
  // Tant que le romaji n'est pas retouché à la main, il suit les kana.
  const [romajiEdited, setRomajiEdited] = useState(vocab?.romaji != null);
  const [kanji, setKanji] = useState(vocab?.kanji ?? "");
  const [meaning, setMeaning] = useState(vocab?.meaning ?? "");
  const [wordType, setWordType] = useState<WordType | "">(vocab?.word_type ?? "");
  const [title, setTitle] = useState(grammar?.title ?? "");
  const [explanation, setExplanation] = useState(grammar?.explanation ?? "");
  const [exampleJp, setExampleJp] = useState(editing?.example_jp ?? "");
  const [exampleFr, setExampleFr] = useState(editing?.example_fr ?? "");
  const [error, setError] = useState<string | null>(null);

  // Pour remettre le curseur dans le premier champ après un ajout.
  const firstField = useRef<HTMLInputElement>(null);

  function handleKanaChange(value: string) {
    setKana(value);
    if (!romajiEdited) setRomaji(toRomaji(value));
  }

  // Construit l'objet attendu par Rust : uniquement les champs du type choisi
  // (deny_unknown_fields refuserait un champ de l'autre type).
  function buildInput(): ItemInput {
    const content: ItemContent =
      kind === "vocab"
        ? {
            kind: "vocab",
            kana: kana.trim(),
            romaji: emptyToNull(romaji),
            kanji: emptyToNull(kanji),
            meaning: meaning.trim(),
            word_type: wordType === "" ? null : wordType,
          }
        : { kind: "grammar", title: title.trim(), explanation: explanation.trim() };
    return {
      lesson_id: lessonId,
      content,
      example_jp: emptyToNull(exampleJp),
      example_fr: emptyToNull(exampleFr),
    };
  }

  // Après un ajout : on vide les champs mais on garde la leçon et le type (saisie en rafale).
  function clearFields() {
    setKana("");
    setRomaji("");
    setRomajiEdited(false);
    setKanji("");
    setMeaning("");
    setWordType("");
    setTitle("");
    setExplanation("");
    setExampleJp("");
    setExampleFr("");
    firstField.current?.focus(); // ?. : ne fait rien si la référence est null
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    try {
      if (editing === null) {
        await api.createItem(buildInput());
        clearFields();
      } else {
        await api.updateItem(editing.id, buildInput());
      }
      setError(null);
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  return (
    <form onSubmit={handleSubmit} className="card space-y-3">
      <div className="flex flex-wrap gap-3">
        <select
          className="field w-auto"
          value={lessonId}
          onChange={(e) => setLessonId(Number(e.target.value))}
        >
          <LessonOptions groups={lessonGroups} />
        </select>
        <div className="flex gap-1">
          {(["vocab", "grammar"] as const).map((k) => (
            <button
              key={k}
              type="button"
              className={kind === k ? "btn-primary" : "btn"}
              onClick={() => setKind(k)}
            >
              {k === "vocab" ? "Vocabulaire" : "Grammaire"}
            </button>
          ))}
        </div>
      </div>

      {kind === "vocab" ? (
        <div className="grid gap-3 sm:grid-cols-2">
          <input
            ref={firstField}
            className="field"
            placeholder="Kana (ex. たべる)"
            value={kana}
            onChange={(e) => handleKanaChange(e.target.value)}
          />
          <input
            className="field"
            placeholder="Traduction (sens séparés par ;)"
            value={meaning}
            onChange={(e) => setMeaning(e.target.value)}
          />
          <input
            className="field"
            placeholder="Romaji (automatique)"
            value={romaji}
            onChange={(e) => {
              setRomaji(e.target.value);
              setRomajiEdited(true);
            }}
          />
          <input
            className="field"
            placeholder="Kanji (facultatif)"
            value={kanji}
            onChange={(e) => setKanji(e.target.value)}
          />
          <select
            className="field"
            value={wordType}
            // `as` : on affirme à TS que la valeur est bien un WordType ou "",
            // ce que garantissent les <option> ci-dessous.
            onChange={(e) => setWordType(e.target.value as WordType | "")}
          >
            <option value="">Type de mot (facultatif)</option>
            {Object.entries(WORD_TYPE_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </div>
      ) : (
        <div className="space-y-3">
          <input
            ref={firstField}
            className="field"
            placeholder="Titre (ex. 〜ています)"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
          <textarea
            className="field"
            rows={3}
            placeholder="Explication"
            value={explanation}
            onChange={(e) => setExplanation(e.target.value)}
          />
        </div>
      )}

      <div className="grid gap-3 sm:grid-cols-2">
        <input
          className="field"
          placeholder="Exemple en japonais (facultatif)"
          value={exampleJp}
          onChange={(e) => setExampleJp(e.target.value)}
        />
        <input
          className="field"
          placeholder="Traduction de l'exemple (facultatif)"
          value={exampleFr}
          onChange={(e) => setExampleFr(e.target.value)}
        />
      </div>

      {error && <p className="error">{error}</p>}

      <div className="flex gap-2">
        <button type="submit" className="btn-primary">
          {editing === null ? "Ajouter (Entrée)" : "Enregistrer"}
        </button>
        {editing !== null && (
          <button type="button" className="btn" onClick={onCancel}>
            Annuler
          </button>
        )}
      </div>
    </form>
  );
}
