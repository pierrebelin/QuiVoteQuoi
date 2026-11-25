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

/** Pastille ronde cochée / vide, à droite des grandes tuiles de choix : blanche, cerclée de gris, puis cyan. */
export function CheckDot({ on }: { on: boolean }) {
  return (
    <span
      aria-hidden
      className={`flex size-[22px] shrink-0 items-center justify-center rounded-full ${
        on ? 'bg-accent text-surface' : 'border-[1.5px] border-line-strong bg-canvas'
      }`}
    >
      {on && <CheckIcon />}
    </span>
  )
}

export function StepIntro({
  step,
  title,
  hint,
  lead,
}: {
  /** Numéro de l'écran, sur deux : « Étape 1 sur 2 » et deux segments dont les pleins sont les écrans atteints. */
  step: 1 | 2
  title: ReactNode
  /** Mode d'emploi : derrière une tape sur téléphone, écrit à partir de 768 px. */
  hint?: ReactNode
  /** Une phrase d'aide assez courte pour rester à l'écran sur téléphone. */
  lead?: ReactNode
}) {
  return (
    <header className="max-w-3xl">
      <p className="flex items-center gap-2.5 text-[13px] font-semibold text-ink-faint md:text-sm">
        <span aria-hidden className="flex gap-1">
          <i className="block h-1 w-[22px] rounded-sm bg-ink" />
          <i className={`block h-1 w-[22px] rounded-sm ${step === 2 ? 'bg-ink' : 'bg-line'}`} />
        </span>
        Étape {step} sur 2
      </p>
      {/* Sous 768 px, l'icône est calée à droite de la rangée du titre. */}
      <div className="mt-3 flex items-start justify-between gap-1 md:block">
        <h2 id="entry-heading" tabIndex={-1} className="heading-2 text-balance outline-none md:heading-1">
          {title}
        </h2>
        {hint && (
          <InfoTip label="En savoir plus" className="-mr-3 md:hidden">
            {hint}
          </InfoTip>
        )}
      </div>
      {lead && <p className="mt-3 text-base leading-normal text-ink-soft md:text-lg md:leading-relaxed">{lead}</p>}
      {hint && (
        <p className="mt-3 hidden text-lg leading-relaxed text-ink-soft md:block">{hint}</p>
      )}
    </header>
  )
}

/**
 * Barre collée en bas de l'écran : le résumé du choix et l'action suivante.
 * Sur téléphone, une carte flottante au-dessus de la barre de navigation, comme
 * elle ; au-delà de 768 px, un bandeau à filet.
 */
export function ActionBar({ summary, children }: { summary: ReactNode; children: ReactNode }) {
  return (
    <div className="sticky bottom-[var(--mobile-nav-h)] z-10 -mx-5 px-3 pt-3 md:-mx-6 md:border-t md:border-line md:bg-canvas/95 md:px-6 md:py-3.5 md:backdrop-blur">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-[20px] border border-line bg-canvas py-2 pr-2 pl-[18px] shadow-float md:rounded-none md:border-0 md:bg-transparent md:p-0 md:shadow-none">
        <p className="text-sm font-semibold text-ink-soft" aria-live="polite">
          {summary}
        </p>
        {children}
      </div>
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
      className={`inline-flex min-h-12 items-center justify-center rounded-[14px] bg-accent px-[18px] text-[15px] font-[620] text-surface transition-colors hover:bg-accent-strong disabled:cursor-not-allowed disabled:bg-surface-soft disabled:text-ink-faint md:min-h-[52px] md:px-[22px] md:text-base ${FOCUS_RING}`}
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
      className={`inline-flex min-h-11 items-center gap-2 rounded-full px-3.5 py-1.5 text-sm font-medium transition-colors disabled:cursor-not-allowed ${FOCUS_RING} ${
        pressed ? 'bg-ink text-surface' : 'bg-surface text-ink hover:bg-surface-soft'
      }`}
    >
      {pressed && <CheckIcon className="size-4 shrink-0" />}
      {children}
    </button>
  )
}
