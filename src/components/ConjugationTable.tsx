// Tableau de conjugaison d'un verbe, calculé par Rust (commande conjugate_verb).
// Utilisé par l'aperçu du formulaire (VerbFields) et par la liste de l'onglet Éléments.

import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { ConjugatedForm, VerbForm, VerbType } from "../types";

// Libellés des formes, dans les cartes et les tableaux.
export const VERB_FORM_LABELS: Record<VerbForm, string> = {
  dictionary: "forme du dictionnaire",
  masu: "forme en ます",
  masen: "forme en ません",
  mashita: "forme en ました",
  masen_deshita: "forme en ませんでした",
  te: "forme en て",
  ta: "forme en た",
  nai: "forme en ない",
  nakatta: "forme en なかった",
};

// Hook : relance la conjugaison à chaque changement de saisie.
// `stale` : une réponse arrivée après une frappe plus récente est ignorée (sinon l'aperçu
// pourrait afficher le tableau d'une saisie précédente).
export function useConjugation(kana: string, kanji: string | null, verbType: VerbType | null) {
  const [forms, setForms] = useState<ConjugatedForm[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (verbType === null || kana.trim() === "") {
      setForms(null);
      setError(null);
      return;
    }
    let stale = false;
    api
      .conjugateVerb(kana, kanji, verbType)
      .then((result) => {
        if (stale) return;
        setForms(result);
        setError(null);
      })
      .catch((e) => {
        if (stale) return;
        setForms(null);
        setError(errorMessage(e));
      });
    return () => {
      stale = true;
    };
  }, [kana, kanji, verbType]);

  return { forms, error };
}

type TableProps = {
  forms: ConjugatedForm[];
  known?: VerbForm[]; // formes connues mises en avant ; absent = toutes affichées pareil
};

export function ConjugationTable({ forms, known }: TableProps) {
  return (
    <table className="text-sm">
      <tbody>
        {forms.map((f) => {
          const isKnown = known === undefined || known.includes(f.form);
          return (
            <tr key={f.form} className={isKnown ? "" : "text-stone-400"}>
              <td className="pr-4">{VERB_FORM_LABELS[f.form]}</td>
              <td className="pr-4 text-base">{f.kanji ?? f.kana}</td>
              <td className="pr-4">{f.kanji !== null && f.kanji !== f.kana ? f.kana : ""}</td>
              <td className="text-xs">{isKnown ? "" : "pas encore"}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

type VerbProps = {
  kana: string;
  kanji: string | null;
  verbType: VerbType;
  known?: VerbForm[];
};

// Tableau d'un verbe déjà enregistré (onglet Éléments).
export function VerbConjugation({ kana, kanji, verbType, known }: VerbProps) {
  const { forms, error } = useConjugation(kana, kanji, verbType);
  if (error) return <p className="text-sm text-amber-700">{error}</p>;
  if (forms === null) return null;
  return <ConjugationTable forms={forms} known={known} />;
}
