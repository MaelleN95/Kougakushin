// Écran des réglages : nouvelles cartes par jour, affichage du romaji.

import { FormEvent, useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { Settings } from "../types";

export default function SettingsPage() {
  // null tant que les réglages ne sont pas chargés.
  const [settings, setSettings] = useState<Settings | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => setError(errorMessage(e)));
  }, []);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (settings === null) return;
    try {
      setSettings(await api.updateSettings(settings));
      setError(null);
      setMessage("Réglages enregistrés.");
    } catch (e) {
      setMessage(null);
      setError(errorMessage(e)); // ex. nombre hors bornes, refusé par Rust
    }
  }

  if (settings === null) {
    return error ? <p className="error">{error}</p> : null;
  }

  return (
    <section className="space-y-6">
      <h2 className="text-xl font-semibold">Réglages</h2>

      <form onSubmit={handleSubmit} className="card space-y-4">
        <label className="block space-y-1">
          <span className="text-sm font-medium">Nouvelles cartes par jour (0 à 100)</span>
          <input
            className="field w-32"
            type="number"
            min={0}
            max={100}
            value={settings.new_cards_per_day}
            onChange={(e) => setSettings({ ...settings, new_cards_per_day: Number(e.target.value) })}
          />
          <span className="block text-xs text-stone-500">
            Chaque nouvelle carte revient les jours suivants : trop de nouvelles cartes par jour
            fait vite grossir les révisions. Le bouton "de plus" en fin de session permet de
            dépasser cette limite ponctuellement.
          </span>
        </label>

        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={settings.show_romaji}
            onChange={(e) => setSettings({ ...settings, show_romaji: e.target.checked })}
          />
          <span className="text-sm">Afficher le romaji (réponses et liste des éléments)</span>
        </label>

        <button type="submit" className="btn-primary">
          Enregistrer
        </button>
      </form>

      {message && <p className="text-sm text-stone-600">{message}</p>}
      {error && <p className="error">{error}</p>}
    </section>
  );
}
