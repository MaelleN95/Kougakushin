// Section "Sauvegarde" des Réglages : export et import JSON de toutes les données.
// L'import remplace tout (D23) : il demande une confirmation en deux clics, et Rust garde une
// copie automatique de l'état d'avant.

import { useState } from "react";
import { api, errorMessage } from "../api";

type Props = {
  onImported: () => void; // pour recharger les réglages affichés
};

export default function BackupSection({ onImported }: Props) {
  const [busy, setBusy] = useState(false);
  const [confirmImport, setConfirmImport] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function exportData() {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const path = await api.exportData();
      if (path !== null) setMessage(`Données exportées dans : ${path}`);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  async function importData() {
    if (!confirmImport) {
      setConfirmImport(true); // premier clic : on arme le bouton
      return;
    }
    setConfirmImport(false);
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const report = await api.importData();
      if (report !== null) {
        setMessage(
          `Import terminé : ${report.lessons} leçon(s), ${report.items} élément(s), ` +
            `${report.cards} carte(s), ${report.review_logs} révision(s). ` +
            `Copie de tes anciennes données : ${report.safety_backup}`,
        );
        onImported();
      }
    } catch (e) {
      setError(errorMessage(e)); // fichier refusé : rien n'a été modifié
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="card space-y-3">
      <h3 className="font-medium">Sauvegarde</h3>
      <p className="text-sm text-stone-600">
        L'export crée un fichier avec toutes tes données (leçons, éléments, progression des
        cartes, historique, réglages). Garde-le ailleurs que sur ce PC.
      </p>
      <div className="flex flex-wrap gap-2">
        <button className="btn" disabled={busy} onClick={exportData}>
          Exporter mes données
        </button>
        <button
          className={confirmImport ? "btn-primary" : "btn-danger"}
          disabled={busy}
          onClick={importData}
          onBlur={() => setConfirmImport(false)} // clic ailleurs : on désarme
        >
          {confirmImport ? "Confirmer : remplacer toutes mes données ?" : "Importer une sauvegarde"}
        </button>
      </div>
      <p className="text-xs text-stone-500">
        L'import remplace toutes les données actuelles. Une copie de l'état actuel est enregistrée
        automatiquement juste avant.
      </p>
      {message && <p className="text-sm text-stone-600 break-all">{message}</p>}
      {error && <p className="error">{error}</p>}
    </div>
  );
}
