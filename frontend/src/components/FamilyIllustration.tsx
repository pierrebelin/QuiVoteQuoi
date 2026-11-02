import type { CSSProperties, ReactNode } from 'react'

/**
 * Vignette d'une famille de thèmes : un pictogramme au trait sur un fond teinté.
 *
 * Dessinés ici plutôt que tirés d'une banque d'images : aucune requête vers un
 * tiers, un rendu net à toute taille, et le mode sombre suit sans second jeu
 * d'images. Les teintes distinguent les familles entre elles, elles ne disent
 * rien du sens des votes — d'où des tons rabattus, loin du vert « pour » et du
 * rouge « contre ».
 */
interface Art {
  hue: string
  glyph: ReactNode
}

const ART: Record<string, Art> = {
  'pouvoir-achat': {
    hue: '#c27c0e',
    glyph: (
      <>
        <path d="M3 4h2l2.3 10.2a1 1 0 0 0 1 .8h8.9a1 1 0 0 0 1-.8L20 8H6.1" />
        <circle cx="9" cy="19" r="1.5" />
        <circle cx="17" cy="19" r="1.5" />
      </>
    ),
  },
  'economie-finances-publiques': {
    hue: '#0f766e',
    glyph: (
      <>
        <path d="M3 20h18" />
        <rect x="5" y="12" width="3" height="8" rx="0.5" />
        <rect x="10.5" y="8" width="3" height="12" rx="0.5" />
        <rect x="16" y="4" width="3" height="16" rx="0.5" />
      </>
    ),
  },
  logement: {
    hue: '#b4532a',
    glyph: (
      <>
        <path d="M3 11l9-7 9 7" />
        <path d="M5.5 9.2V20h13V9.2" />
        <path d="M10 20v-5h4v5" />
      </>
    ),
  },
  'travail-emploi': {
    hue: '#4f46e5',
    glyph: (
      <>
        <rect x="3" y="7" width="18" height="13" rx="2" />
        <path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
        <path d="M3 13h18" />
      </>
    ),
  },
  'protection-sociale-retraites': {
    hue: '#0e7490',
    glyph: (
      <>
        <path d="M3 12a9 9 0 0 1 18 0z" />
        <path d="M12 3V2" />
        <path d="M12 12v6.5a2 2 0 0 1-4 0" />
      </>
    ),
  },
  sante: {
    hue: '#be4b6c',
    glyph: <path d="M9 3.5h6v5.5h5.5v6H15v5.5H9V15H3.5V9H9z" />,
  },
  'environnement-energie': {
    hue: '#4d7c0f',
    glyph: (
      <>
        <path d="M20 4C10 4 5 9 5 15.5c0 1.6.4 3 1 4C13.5 19.5 20 14.5 20 4z" />
        <path d="M4 21c2.5-5 6.5-9 11-12" />
      </>
    ),
  },
  'agriculture-alimentation': {
    hue: '#a16207',
    glyph: (
      <>
        <path d="M12 7v14" />
        <path d="M12 6c-1.2-1.2-1.2-3 0-4.2 1.2 1.2 1.2 3 0 4.2z" />
        <path d="M12 11c-2.5 0-4-1.5-4-4 2.5 0 4 1.5 4 4z" />
        <path d="M12 11c2.5 0 4-1.5 4-4-2.5 0-4 1.5-4 4z" />
        <path d="M12 15.5c-2.5 0-4-1.5-4-4 2.5 0 4 1.5 4 4z" />
        <path d="M12 15.5c2.5 0 4-1.5 4-4-2.5 0-4 1.5-4 4z" />
      </>
    ),
  },
  numerique: {
    hue: '#7c3aed',
    glyph: (
      <>
        <rect x="3" y="4" width="18" height="12" rx="1.5" />
        <path d="M8 20h8" />
        <path d="M12 16v4" />
        <path d="M10 8l-2 2 2 2" />
        <path d="M14 8l2 2-2 2" />
      </>
    ),
  },
  'justice-securite': {
    hue: '#475569',
    glyph: (
      <>
        <path d="M12 4v16" />
        <path d="M8 20h8" />
        <path d="M5 7h14" />
        <path d="M2 13l3-6 3 6a3 3 0 0 1-6 0z" />
        <path d="M16 13l3-6 3 6a3 3 0 0 1-6 0z" />
      </>
    ),
  },
  immigration: {
    hue: '#0369a1',
    glyph: (
      <>
        <rect x="5" y="3" width="14" height="18" rx="2" />
        <circle cx="12" cy="10" r="3.5" />
        <path d="M8.5 10h7" />
        <path d="M12 6.5c-1 1-1.5 2.2-1.5 3.5s.5 2.5 1.5 3.5c1-1 1.5-2.2 1.5-3.5S13 7.5 12 6.5z" />
        <path d="M9 17.5h6" />
      </>
    ),
  },
  'education-culture': {
    hue: '#a21caf',
    glyph: (
      <>
        <path d="M3 5h5.5A3.5 3.5 0 0 1 12 8.5V20a2.5 2.5 0 0 0-2.5-2.5H3z" />
        <path d="M21 5h-5.5A3.5 3.5 0 0 0 12 8.5V20a2.5 2.5 0 0 1 2.5-2.5H21z" />
      </>
    ),
  },
  'societe-libertes': {
    hue: '#c2410c',
    glyph: (
      <>
        <circle cx="9" cy="8" r="3" />
        <circle cx="17" cy="9" r="2.5" />
        <path d="M3 20a6 6 0 0 1 12 0" />
        <path d="M15 14.6a4.5 4.5 0 0 1 6 4.4" />
      </>
    ),
  },
  'international-defense': {
    hue: '#3f6212',
    glyph: (
      <>
        <path d="M12 3l7 3v5c0 5-3 8.5-7 10-4-1.5-7-5-7-10V6z" />
        <path d="M12 8.2l1 2.1 2.3.3-1.7 1.6.4 2.3-2-1.1-2 1.1.4-2.3-1.7-1.6 2.3-.3z" />
      </>
    ),
  },
  'institutions-vie-politique': {
    hue: '#1d4ed8',
    glyph: (
      <>
        <path d="M3 9l9-5 9 5" />
        <path d="M4 9h16" />
        <path d="M6 9v8M10 9v8M14 9v8M18 9v8" />
        <path d="M4 17h16" />
        <path d="M3 20h18" />
      </>
    ),
  },
  'collectivites-outre-mer': {
    hue: '#0f766e',
    glyph: (
      <>
        <path d="M12 21s-7-6.2-7-12a7 7 0 0 1 14 0c0 5.8-7 12-7 12z" />
        <circle cx="12" cy="9" r="2.5" />
      </>
    ),
  },
}

// Famille ajoutée au référentiel sans vignette : une étiquette neutre plutôt
// qu'une case vide.
const FALLBACK: Art = {
  hue: '#52525b',
  glyph: (
    <>
      <path d="M3 12V4h8l10 10-7 7z" />
      <circle cx="7.5" cy="8.5" r="1.5" />
    </>
  ),
}

export function FamilyIllustration({
  code,
  className = '',
}: {
  code: string
  className?: string
}) {
  const art = ART[code] ?? FALLBACK
  return (
    <div
      aria-hidden
      style={{ '--family-hue': art.hue } as CSSProperties}
      className={`family-art flex items-center justify-center ${className}`}
    >
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
        className="size-9 transition-transform duration-300 group-hover:scale-110"
      >
        {art.glyph}
      </svg>
    </div>
  )
}
