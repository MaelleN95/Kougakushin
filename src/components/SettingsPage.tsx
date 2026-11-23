// Écran des réglages : nouvelles cartes par jour, romaji, modèle d'IA et test de connexion à Ollama.

import { FormEvent, useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { OllamaStatus, Settings } from "../types";

export default function SettingsPage() {
  // null tant que les réglages ne sont pas chargés.
  const [settings, setSettings] = useState<Settings | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Test de connexion : null = pas encore testé ; ollamaError = Ollama injoignable.
  const [ollama, setOllama] = useState<OllamaStatus | null>(null);
  const [ollamaError, setOllamaError] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);

  async function testOllama() {
    setTesting(true);
    try {
      setOllama(await api.checkOllama());
      setOllamaError(null);
    } catch (e) {
      setOllama(null);
      setOllamaError(errorMessage(e));
    } finally {
      setTesting(false);
    }
  }

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => setError(errorMessage(e)));
    testOllama(); // remplit la liste des modèles dès l'ouverture
  }, []);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (settings === null) return;
    try {
      setSettings(await api.updateSettings(settings));
      setError(null);
      setMessage("Réglages enregistrés.");
      testOllama(); // le modèle choisi a pu changer
    } catch (e) {
      setMessage(null);
      setError(errorMessage(e)); // ex. nombre hors bornes, refusé par Rust
    }
  }

  if (settings === null) {
    return error ? <p className="error">{error}</p> : null;
  }

  // Modèles proposés : ceux d'Ollama, plus le modèle enregistré s'il n'est pas installé.
  const models = ollama?.models ?? [];
  const withCurrent = (current: string) =>
    models.includes(current) ? models : [current, ...models];
  const modelOptions = withCurrent(settings.ollama_model);
  const translatorOptions = withCurrent(settings.translation_model);

  return (
    <section className="space-y-6">
      <h2 className="text-xl font-semibold">Réglages</h2>

      <form onSubmit={handleSubmit} className="card space-y-4">
        <h3 className="font-medium">Révisions</h3>
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

        <h3 className="border-t border-stone-200 pt-4 font-medium">Intelligence artificielle</h3>
        <label className="block space-y-1">
          <span className="text-sm font-medium">Modèle Ollama</span>
          <select
            className="field w-auto"
            value={settings.ollama_model}
            onChange={(e) => setSettings({ ...settings, ollama_model: e.target.value })}
          >
            {modelOptions.map((model) => (
              <option key={model} value={model}>
                {model}
                {!models.includes(model) && " (non installé)"}
              </option>
            ))}
          </select>
        </label>

        <label className="block space-y-1">
          <span className="text-sm font-medium">Traducteur (japonais vers français)</span>
          <select
            className="field w-auto"
            value={settings.translation_model}
            onChange={(e) => setSettings({ ...settings, translation_model: e.target.value })}
          >
            {translatorOptions.map((model) => (
              <option key={model} value={model}>
                {model}
                {!models.includes(model) && " (non installé)"}
              </option>
            ))}
          </select>
          <span className="block text-xs text-stone-500">
            Traduit les phrases des exercices et les messages de la conversation. Il tourne sur le
            processeur, pour laisser la carte graphique au modèle principal.
          </span>
        </label>

        <div className="space-y-1">
          <button type="button" className="btn" disabled={testing} onClick={testOllama}>
            {testing ? "Test en cours..." : "Tester la connexion"}
          </button>
          {ollama && (
            <p className="text-sm text-stone-600">
              Ollama {ollama.version} répond, {ollama.models.length} modèle(s) installé(s).
              {!ollama.configured_model_installed &&
                " Le modèle enregistré n'est pas installé : choisis-en un autre ou télécharge-le."}
              {!ollama.translation_model_installed &&
                " Le traducteur enregistré n'est pas installé : choisis-en un autre ou télécharge-le."}
            </p>
          )}
          {ollamaError && <p className="error">{ollamaError}</p>}
        </div>

        <button type="submit" className="btn-primary">
          Enregistrer
        </button>
      </form>

      {message && <p className="text-sm text-stone-600">{message}</p>}
      {error && <p className="error">{error}</p>}
    </section>
  );
}
