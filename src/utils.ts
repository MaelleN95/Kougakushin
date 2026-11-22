// Petites fonctions partagées par les écrans.

// Date du jour en AAAA-MM-JJ, en heure locale.
// Pas toISOString() : il donne la date UTC, donc la veille entre minuit et 1h en France.
export function todayLocal(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

// Champ facultatif : texte vide ou espaces => null (NULL en base).
export function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

// Copie mélangée d'une liste (Fisher-Yates) : évite de voir les deux sens d'un mot à la suite.
export function shuffled<T>(list: T[]): T[] {
  const copy = [...list];
  for (let i = copy.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [copy[i], copy[j]] = [copy[j], copy[i]];
  }
  return copy;
}
