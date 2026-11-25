// Section "Formes verbales connues" des Réglages (D40, D41) : un interrupteur par forme.
// Une forme connue est utilisée par l'IA et a ses flashcards de conjugaison.
// Chaque clic est enregistré tout de suite (pas besoin du bouton "Enregistrer" des réglages).

import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { ConjugationFormState, VerbForm } from "../types";
import { VERB_FORM_LABELS } from "./ConjugationTable";

export default function KnownFormsSection() {
  const [forms, setForms] = useState<ConjugationFormState[]>([]);
  // Forme dont la désactivation attend une confirmation (elle supprime des cartes).
  const [pendingOff, setPendingOff] = useState<VerbForm | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listConjugationForms().then(setForms).catch((e) => setError(errorMessage(e)));
  }, []);

  async function save(form: VerbForm, known: boolean) {
    setBusy(true);
    setPendingOff(null);
    try {
      setForms(await api.setConjugationFormKnown(form, known));
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  function toggle(state: ConjugationFormState) {
    if (state.known) {
      setPendingOff(state.form); // désactiver : confirmation d'abord
    } else {
      save(state.form, true);
    }
  }

  return (
    <div className="card space-y-3">
      <h3 className="font-medium">Formes verbales connues</h3>
      <p className="text-xs text-stone-500">
        L'IA ne conjugue les verbes qu'avec les formes cochées, et chaque forme cochée crée une
        flashcard par verbe (ex. "たべる - forme en て ?").
      </p>
      <ul className="grid gap-1 sm:grid-cols-2">
        {forms.map((state) => (
          <li key={state.form}>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={state.known}
                disabled={busy}
                onChange={() => toggle(state)}
              />
              {VERB_FORM_LABELS[state.form]}
              <span className="text-stone-400">{state.known ? "je connais" : "pas encore"}</span>
            </label>
          </li>
        ))}
      </ul>
      {pendingOff !== null && (
        <div className="flex flex-wrap items-center gap-2 text-sm">
          <span>
            Désactiver la {VERB_FORM_LABELS[pendingOff]} supprime ses flashcards et leur
            progression.
          </span>
          <button className="btn-danger" disabled={busy} onClick={() => save(pendingOff, false)}>
            Confirmer
          </button>
          <button className="btn" onClick={() => setPendingOff(null)}>
            Annuler
          </button>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </div>
  );
}
