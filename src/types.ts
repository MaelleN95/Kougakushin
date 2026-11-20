// Les typages de TS se font généralement par convention dans un fichier à part, ici types.ts, mais il est possible de le faire dans le fichier où on en a besoin. Cependant, il est préférable de les mettre dans un fichier à part pour que ce soit plus lisible et plus facile à maintenir.

// Je type l'objet de sortie JSON de la fonction analyser_mot comme en Rust mais pour TS
export type AnalyseDuMot = {
  octets: number;
  caracteres: number;
};