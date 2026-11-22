/**
 * Gabarit commun aux quatre compositions (RM-11) : mêmes couleurs, même police,
 * mêmes zones réservées, quels que soient les groupes ou candidats concernés.
 */
export const WIDTH = 1080
export const HEIGHT = 1920
export const FPS = 30

/**
 * Zones couvertes par l'interface d'Instagram et de TikTok (nom du compte en
 * haut, légende et boutons en bas, icônes à droite) : celles du gabarit
 * `vertical` de `frontend/src/lib/voteImage.ts`.
 */
export const SAFE = { top: 250, bottom: 440, left: 72, right: 140 }

/** Palette claire de `voteImage.ts` (tokens de `index.css`). */
export const COLORS = {
  background: '#ffffff',
  backgroundSoft: '#f4f4f5',
  line: '#e8e8ea',
  ink: '#18181b',
  inkSoft: '#52525b',
  inkFaint: '#8e8e97',
  accent: '#0e7490',
  adopted: '#04785d',
  rejected: '#b3261e',
  other: '#9a5b06',
}

export const FONT = '"Inter Variable", ui-sans-serif, system-ui, sans-serif'
