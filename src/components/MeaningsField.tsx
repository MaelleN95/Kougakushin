// Saisie des sens français d'un mot (D43) : un champ par sens, "+" pour en ajouter un.
// Entrée dans n'importe quel champ soumet le formulaire parent, comme les autres champs
// (saisie en rafale) : les boutons sont en type="button" pour ne pas intercepter Entrée.
// Le nettoyage (espaces, sens vides, doublons) est fait par Rust.

import { useEffect, useRef } from "react";

type Props = {
  meanings: string[]; // au moins un élément (éventuellement vide) pour afficher un champ
  onChange: (meanings: string[]) => void;
};

export default function MeaningsField({ meanings, onChange }: Props) {
  // Un champ par sens, pour placer le curseur dans celui qu'on vient d'ajouter.
  const inputs = useRef<(HTMLInputElement | null)[]>([]);
  const focusNew = useRef(false);

  // Après "+" : le nouveau champ n'existe qu'au rendu suivant, d'où l'effet.
  useEffect(() => {
    if (focusNew.current) {
      inputs.current[meanings.length - 1]?.focus();
      focusNew.current = false;
    }
  }, [meanings.length]);

  function update(index: number, value: string) {
    onChange(meanings.map((meaning, i) => (i === index ? value : meaning)));
  }

  function add() {
    focusNew.current = true;
    onChange([...meanings, ""]);
  }

  function remove(index: number) {
    onChange(meanings.filter((_, i) => i !== index));
  }

  return (
    <div className="space-y-2">
      {meanings.map((meaning, index) => (
        // Index en clé : les champs n'ont pas d'identifiant, et l'ordre ne change que par ajout
        // en fin de liste ou retrait, ce qui suffit à React ici.
        <div key={index} className="flex gap-1">
          <input
            ref={(element) => {
              inputs.current[index] = element;
            }}
            className="field"
            placeholder={index === 0 ? "Traduction (ex. manger)" : "Autre sens"}
            value={meaning}
            onChange={(e) => update(index, e.target.value)}
          />
          {meanings.length > 1 && (
            <button
              type="button"
              className="btn shrink-0"
              aria-label="Retirer ce sens"
              title="Retirer ce sens"
              onClick={() => remove(index)}
            >
              ×
            </button>
          )}
          {index === meanings.length - 1 && (
            <button
              type="button"
              className="btn shrink-0"
              aria-label="Ajouter un sens"
              title="Ajouter un sens"
              onClick={add}
            >
              +
            </button>
          )}
        </div>
      ))}
    </div>
  );
}
