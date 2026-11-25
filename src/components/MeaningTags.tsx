// Affichage des sens français d'un mot (D43) : une étiquette par sens, dans l'ordre de saisie.

type Props = {
  meanings: string[];
  // "sm" : liste des éléments ; "md" : réponse d'une flashcard ; "lg" : question fr => jp.
  size?: "sm" | "md" | "lg";
  centered?: boolean; // flashcards : étiquettes centrées
};

const SIZE_CLASSES = {
  sm: "px-2 py-0.5 text-sm",
  md: "px-3 py-1 text-lg",
  lg: "px-4 py-1.5 text-3xl",
} as const;

export default function MeaningTags({ meanings, size = "sm", centered = false }: Props) {
  return (
    <ul className={`flex flex-wrap gap-1.5 ${centered ? "justify-center" : ""}`}>
      {meanings.map((meaning) => (
        // Rust garantit des sens distincts : le texte suffit comme clé.
        <li
          key={meaning}
          className={`rounded-full border border-stone-200 bg-stone-100 text-stone-800 ${SIZE_CLASSES[size]}`}
        >
          {meaning}
        </li>
      ))}
    </ul>
  );
}
