// Barre d'actions sur les éléments cochés : activer, désactiver, changer de leçon, supprimer.

import { useState } from "react";
import type { LessonGroup } from "../lessonGroups";
import LessonOptions from "./LessonOptions";

type Props = {
  count: number; // éléments cochés
  visibleCount: number; // éléments affichés (filtre en cours)
  lessonGroups: LessonGroup[];
  busy: boolean;
  onSelectAll: () => void;
  onClear: () => void;
  onSetActive: (active: boolean) => void;
  onMove: (lessonId: number) => void;
  onDelete: () => void;
};

export default function SelectionBar(props: Props) {
  const { count, visibleCount, lessonGroups, busy } = props;
  const [target, setTarget] = useState(""); // leçon de destination, "" = pas choisie
  const [confirmDelete, setConfirmDelete] = useState(false);

  function handleDelete() {
    if (!confirmDelete) {
      setConfirmDelete(true); // premier clic : on arme le bouton
      return;
    }
    setConfirmDelete(false);
    props.onDelete();
  }

  return (
    <div className="card flex flex-wrap items-center gap-2">
      <span className="text-sm font-medium">
        {count} sélectionné(s) sur {visibleCount}
      </span>
      {count < visibleCount && (
        <button className="btn" onClick={props.onSelectAll}>
          Tout sélectionner
        </button>
      )}
      {count > 0 && (
        <>
          <button className="btn" onClick={props.onClear}>
            Désélectionner
          </button>
          <span className="mx-1 h-6 border-l border-stone-300" />
          <button className="btn" disabled={busy} onClick={() => props.onSetActive(true)}>
            Activer
          </button>
          <button className="btn" disabled={busy} onClick={() => props.onSetActive(false)}>
            Désactiver
          </button>
          <select
            className="field w-auto"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
          >
            <option value="">Changer de leçon...</option>
            <LessonOptions groups={lessonGroups} />
          </select>
          <button
            className="btn"
            disabled={busy || target === ""}
            onClick={() => {
              props.onMove(Number(target));
              setTarget("");
            }}
          >
            Déplacer
          </button>
          <button
            className={confirmDelete ? "btn-primary" : "btn-danger"}
            disabled={busy}
            onClick={handleDelete}
            onBlur={() => setConfirmDelete(false)} // clic ailleurs : on désarme
          >
            {confirmDelete ? `Confirmer : supprimer ${count} élément(s) ?` : "Supprimer"}
          </button>
        </>
      )}
    </div>
  );
}
