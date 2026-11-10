import type { ReactNode } from 'react'
import { InfoTip } from '../InfoTip'
import { FOCUS_RING } from './styles'

/*
 * Pièces communes aux trois écrans du parcours d'accueil.
 *
 * Tout ce qui se touche est un vrai <button> ou <a>, haut d'au moins 44 px, avec
 * un contour de focus visible au clavier.
 */

export function CheckIcon({ className = 'size-3.5' }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={3.2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={className}
    >
      <path d="M5 12.5l4.5 4.5L19 7.5" />
    </svg>
  )
}

/** Pastille ronde cochée / vide, à droite des grandes tuiles de choix. */
export function CheckDot({ on }: { on: boolean }) {
  return (
    <span
      aria-hidden
      className={`flex size-[22px] shrink-0 items-center justify-center rounded-full ${
        on ? 'bg-accent text-surface' : 'border-2 border-line-strong'
      }`}
    >
      {on && <CheckIcon />}
    </span>
  )
}

export function StepIntro({
  eyebrow,
  title,
  hint,
  lead,
}: {
  eyebrow: string
  title: string
  /** Mode d'emploi : derrière une tape sur téléphone, écrit à partir de 768 px. */
  hint?: ReactNode
  /** Une phrase d'aide assez courte pour rester à l'écran sur téléphone. */
  lead?: ReactNode
}) {
  return (
    <header className="max-w-3xl">
      <p className="text-xs font-semibold uppercase tracking-[0.08em] text-accent">{eyebrow}</p>
      <h2
        id="entry-heading"
        tabIndex={-1}
        className="mt-2 text-3xl leading-[1.15] font-semibold tracking-tight outline-none sm:text-4xl"
      >
        {title}
        {hint && (
          <InfoTip label="En savoir plus" className="md:hidden">
            {hint}
          </InfoTip>
        )}
      </h2>
      {lead && <p className="mt-2 text-sm leading-relaxed text-ink-soft sm:text-base">{lead}</p>}
      {hint && (
        <p className="mt-3 hidden text-base leading-relaxed text-ink-soft md:block sm:text-[17px]">{hint}</p>
      )}
    </header>
  )
}

/** Barre collée en bas de l'écran : le résumé du choix et l'action suivante. */
export function ActionBar({ summary, children }: { summary: ReactNode; children: ReactNode }) {
  return (
    <div className="sticky bottom-[var(--mobile-nav-h)] z-10 -mx-6 flex flex-wrap items-center justify-between gap-3 border-t border-line bg-canvas/95 px-6 py-3.5 backdrop-blur">
      <p className="text-sm text-ink-soft" aria-live="polite">
        {summary}
      </p>
      {children}
    </div>
  )
}

export function PrimaryButton({
  children,
  onClick,
  disabled = false,
}: {
  children: ReactNode
  onClick: () => void
  disabled?: boolean
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`inline-flex min-h-12 items-center justify-center rounded-xl bg-accent px-6 text-base font-semibold text-surface transition-colors hover:bg-accent-strong disabled:cursor-not-allowed disabled:bg-surface-soft disabled:text-ink-faint ${FOCUS_RING}`}
    >
      {children}
    </button>
  )
}

/**
 * Pastille à bascule des panneaux de la barre des votes. L'état actif se lit à
 * la coche autant qu'à l'inversion des couleurs.
 */
export function ToggleChip({
  pressed,
  onClick,
  children,
  disabled = false,
  title,
}: {
  pressed: boolean
  onClick: () => void
  children: ReactNode
  disabled?: boolean
  title?: string
}) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      disabled={disabled}
      title={title}
      className={`inline-flex min-h-11 items-center gap-2 rounded-full border px-3.5 py-1.5 text-sm font-medium transition-colors disabled:cursor-not-allowed ${FOCUS_RING} ${
        pressed
          ? 'border-ink bg-ink text-surface'
          : 'border-line-strong bg-surface text-ink hover:bg-surface-soft'
      }`}
    >
      {pressed && <CheckIcon className="size-4 shrink-0" />}
      {children}
    </button>
  )
}
