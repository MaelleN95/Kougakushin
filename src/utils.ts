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
