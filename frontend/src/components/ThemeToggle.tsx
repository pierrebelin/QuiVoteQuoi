import { useEffect, useState } from 'react'

type Theme = 'light' | 'dark'

const CLE = 'theme'

/** Couleur de `--color-surface` dans chaque apparence : celle de l'en-tête. */
const COULEUR_BARRE: Record<Theme, string> = {
  light: '#ffffff',
  dark: '#1b1a18',
}

/**
 * Le script de `index.html` a déjà posé l'attribut avant le premier rendu.
 * On le relit plutôt que de recalculer la préférence, pour que le bouton
 * affiche exactement ce que la page montre.
 */
function themeCourant(): Theme {
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light'
}

/**
 * `icon` : bouton carré de la barre haute. `row` : ligne pleine largeur du
 * tiroir « Plus », haute d'au moins 44 px, avec son libellé écrit.
 */
export default function ThemeToggle({ variant = 'icon' }: { variant?: 'icon' | 'row' }) {
  const [theme, setTheme] = useState<Theme>(themeCourant)

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    document
      .querySelector('meta[name="theme-color"]')
      ?.setAttribute('content', COULEUR_BARRE[theme])
    try {
      localStorage.setItem(CLE, theme)
    } catch {
      // Stockage refusé (navigation privée) : le thème reste valable pour la
      // session, la préférence système reprend la main au rechargement.
    }
  }, [theme])

  const sombre = theme === 'dark'
  const row = variant === 'row'
  const label = sombre ? 'Passer au thème clair' : 'Passer au thème sombre'
  const glyphSize = row ? 'size-5' : 'size-4'

  const glyph = sombre ? (
    <svg viewBox="0 0 24 24" className={`${glyphSize} shrink-0`} aria-hidden>
      <circle cx="12" cy="12" r="4.2" fill="currentColor" />
      {[0, 45, 90, 135, 180, 225, 270, 315].map((angle) => (
        <line
          key={angle}
          x1="12"
          y1="2.6"
          x2="12"
          y2="5.2"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
          transform={`rotate(${angle} 12 12)`}
        />
      ))}
    </svg>
  ) : (
    <svg viewBox="0 0 24 24" className={`${glyphSize} shrink-0`} aria-hidden>
      <path d="M20 14.2A8.4 8.4 0 1 1 9.8 4a6.8 6.8 0 0 0 10.2 10.2Z" fill="currentColor" />
    </svg>
  )

  if (row) {
    return (
      <button
        type="button"
        onClick={() => setTheme(sombre ? 'light' : 'dark')}
        className="flex min-h-[50px] w-full items-center gap-3 border-t border-line px-4 text-left text-[15px] font-medium text-ink transition-colors hover:bg-surface-soft focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent"
      >
        {glyph}
        {label}
      </button>
    )
  }

  return (
    <button
      type="button"
      onClick={() => setTheme(sombre ? 'light' : 'dark')}
      aria-label={label}
      title={sombre ? 'Thème clair' : 'Thème sombre'}
      className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded text-ink-soft transition-colors hover:bg-surface-soft hover:text-ink"
    >
      {glyph}
    </button>
  )
}
