/**
 * Mention affichée quand le fichier de l'Assemblée range un vote sous un autre
 * texte que celui auquel il est rattaché ici. L'écart se dit, il ne se corrige
 * pas en silence.
 */
export const REATTACHED_LABEL = 'Rattachement corrigé'

export function reattachedNote(sourceDossierLabel: string): string {
  return `Le fichier de l’Assemblée range ce vote sous « ${sourceDossierLabel} ». Il est rattaché ici au texte que son intitulé nomme.`
}
