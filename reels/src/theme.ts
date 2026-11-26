/**
 * Gabarit commun aux quatre compositions (RM-11) : mêmes couleurs, même police,
 * mêmes zones réservées, quels que soient les groupes ou candidats concernés.
 */
export const WIDTH = 1080
export const HEIGHT = 1920
export const FPS = 30

/** Post du jour : carrousel Instagram en 4:5, une image par scène du Reel. */
export const POST_HEIGHT = 1350
/** Images d'un carrousel Instagram, au plus. */
export const POST_MAX_IMAGES = 20

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
/** Titres et nombres : la police de titre du site (`--font-heading` de `index.css`). */
export const FONT_DISPLAY = `"Bricolage Grotesque Variable", ${FONT}`

/**
 * Palette du Reel de votes et de la carte finale : le bleu du logo en fond,
 * les tokens du thème sombre de `index.css` pour le reste.
 */
export const NIGHT = {
  background: '#16233f',
  backgroundDeep: '#0e1216',
  ink: '#eef1f4',
  inkSoft: '#b4bec8',
  accent: '#3fb6d0',
  glow: 'rgba(14, 116, 144, 0.42)',
  panel: '#29354e',
  chip: 'rgba(255, 255, 255, 0.1)',
  line: 'rgba(255, 255, 255, 0.16)',
  seatEmpty: 'rgba(255, 255, 255, 0.16)',
  notVoting: '#a1a1aa',
  for: '#34d399',
  against: '#fca5a5',
  abstention: '#fbbf24',
  /** Fonds doux `--color-yes-soft`, `--color-no-soft`, `--color-abstain-soft`, aplatis sur le fond. */
  forSoft: '#104047',
  againstSoft: '#432033',
  abstentionSoft: '#46372a',
}
