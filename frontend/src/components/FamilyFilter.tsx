import type { ReactNode } from 'react'
import { FamilyIllustration } from './FamilyIllustration'
import { Icon } from './icons'
import { FOCUS, SheetSection } from './SheetControls'
import type { FamilyDto } from '../types/themes'

/**
 * Choix d'un sujet dans un tiroir de filtres. Des tuiles plutôt qu'une liste
 * déroulante : on reconnaît un sujet à son pictogramme avant d'en lire le
 * libellé. Le sujet choisi est plein et coché : il ne se distingue pas par la
 * seule couleur.
 */
export default function FamilyFilter({
  families,
  theme,
  onTheme,
}: {
  families: FamilyDto[]
  theme: string
  /** Code du sujet à filtrer, ou `''` pour tous. */
  onTheme: (code: string) => void
}) {
  return (
    <SheetSection title="Sujet">
      <div role="group" aria-label="Sujet" className="grid grid-cols-2 gap-1.5">
        <ThemeTile active={!theme} onClick={() => onTheme('')}>
          <span className="min-w-0">Tous les sujets</span>
        </ThemeTile>
        {families.map((family) => {
          const on = theme === family.code
          return (
            <ThemeTile
              key={family.code}
              active={on}
              title={family.scope}
              onClick={() => onTheme(on ? '' : family.code)}
              icon={
                on ? undefined : (
                  <FamilyIllustration code={family.code} className="size-8 rounded" glyphClassName="size-5" />
                )
              }
            >
              <span className="min-w-0">{family.label}</span>
            </ThemeTile>
          )
        })}
      </div>
    </SheetSection>
  )
}

/** Tuile de sujet de 48 px : pictogramme (ou coche si choisi) puis libellé. */
function ThemeTile({
  active,
  onClick,
  title,
  icon,
  children,
}: {
  active: boolean
  onClick: () => void
  title?: string
  icon?: ReactNode
  children: ReactNode
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      title={title}
      className={`flex min-h-12 min-w-0 items-center gap-2 rounded py-1 pr-2 ${
        active || icon ? 'pl-1.5' : 'pl-3'
      } text-left text-[12.5px] leading-tight transition-colors ${FOCUS} ${
        active
          ? 'bg-ink font-semibold text-surface'
          : 'font-medium text-ink-soft ring-1 ring-line ring-inset hover:text-ink hover:ring-line-strong'
      }`}
    >
      {active ? (
        <span className="flex size-8 shrink-0 items-center justify-center">
          <Icon name="check" className="size-4" strokeWidth={3} />
        </span>
      ) : (
        icon
      )}
      {children}
    </button>
  )
}
