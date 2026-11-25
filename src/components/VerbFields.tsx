// Champs propres au mode "Verbe" du formulaire (D42) : type de verbe obligatoire et aperçu du
// tableau de conjugaison en direct. En base, un verbe reste un élément de vocabulaire.

import type { VerbType, WordType } from "../types";
import { ConjugationTable, useConjugation } from "./ConjugationTable";

export const VERB_TYPE_LABELS: Record<VerbType, string> = {
  ru_verb: "Verbe en -ru (たべる, みる)",
  u_verb: "Verbe en -u (かう, よむ, いく)",
  irregular_verb: "Verbe irrégulier (する, くる, べんきょうする)",
};

// Garde de type : `value is VerbType` dit à TS que, si la fonction renvoie true, la valeur est
// un VerbType (comme un `instanceof`, mais pour une union de chaînes).
export function isVerbType(value: WordType | "" | null | undefined): value is VerbType {
  return value === "ru_verb" || value === "u_verb" || value === "irregular_verb";
}

type Props = {
  kana: string;
  kanji: string; // "" = pas de kanji
  wordType: WordType | "";
  onWordTypeChange: (value: VerbType | "") => void;
};

export default function VerbFields({ kana, kanji, wordType, onWordTypeChange }: Props) {
  const verbType = isVerbType(wordType) ? wordType : null;
  const { forms, error } = useConjugation(kana, kanji.trim() === "" ? null : kanji, verbType);

  return (
    <div className="space-y-2">
      <select
        className="field"
        required // le navigateur refuse l'envoi sans type (en plus de la vérification d'ItemForm)
        value={verbType ?? ""}
        onChange={(e) => onWordTypeChange(e.target.value as VerbType | "")}
      >
        <option value="">Type de verbe (obligatoire)</option>
        {Object.entries(VERB_TYPE_LABELS).map(([value, label]) => (
          <option key={value} value={value}>
            {label}
          </option>
        ))}
      </select>
      {error && <p className="text-sm text-amber-700">{error}</p>}
      {forms && (
        <div className="rounded-md bg-stone-50 p-3">
          <p className="mb-1 text-xs uppercase tracking-wide text-stone-400">Aperçu de la conjugaison</p>
          <ConjugationTable forms={forms} />
        </div>
      )}
    </div>
  );
}
