import type { ReactNode } from 'react'
import { Icon } from './icons'

/**
 * Contrôles communs aux tiroirs de choix (`BottomSheet`) et aux panneaux de
 * filtres qui reprennent les mêmes contrôles à grande largeur : mêmes cibles de
 * 44 px au moins, même façon de montrer qu'un choix est fait.
 *
 * Un choix fait n'est jamais dit par la seule couleur : la case est pleine ET
 * cochée, la tuile est pleine ET cochée. Seules les couleurs sémantiques du
 * thème servent ici, le sombre suit sans règle propre.
 */

export const FOCUS =
  'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

/** Intertitre d'une section du tiroir (« Sort », « Thème »…). */
export function SheetSection({
  title,
  children,
}: {
  title: string
  children: ReactNode
}) {
  return (
    <section className="mt-4 first:mt-1">
      <h3 className="mb-1.5 text-xs font-semibold tracking-wide text-ink-soft uppercase">
        {title}
      </h3>
      {children}
    </section>
  )
}

/** Case à cocher en rangée de 48 px : toute la rangée se touche, pas la case seule. */
export function CheckRow({
  checked,
  onToggle,
  disabled = false,
  children,
}: {
  checked: boolean
  onToggle: () => void
  disabled?: boolean
  children: ReactNode
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      disabled={disabled}
      onClick={onToggle}
      className={`flex min-h-12 w-full items-center gap-3 border-b border-line px-0.5 text-left text-[15px] text-ink transition-colors hover:bg-surface-soft disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent ${FOCUS} ${
        checked ? 'font-semibold' : 'font-medium'
      }`}
    >
      <span
        aria-hidden
        className={`flex size-6 shrink-0 items-center justify-center rounded-[7px] ${
          checked ? 'bg-ink text-surface' : 'ring-[1.5px] ring-line-strong ring-inset'
        }`}
      >
        {checked && <Icon name="check" className="size-[15px]" strokeWidth={3.2} />}
      </span>
      {children}
    </button>
  )
}

/** Bouton du pied de tiroir : 44 px, plein pour l'action qui referme. */
export function SheetButton({
  children,
  onClick,
  primary = false,
  disabled = false,
}: {
  children: ReactNode
  onClick: () => void
  primary?: boolean
  disabled?: boolean
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`inline-flex min-h-11 items-center justify-center rounded px-4 text-sm font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${FOCUS} ${
        primary
          ? 'min-w-0 flex-1 bg-ink text-surface'
          : 'bg-surface text-ink ring-1 ring-line-strong ring-inset hover:bg-surface-soft disabled:hover:bg-surface'
      }`}
    >
      {children}
    </button>
  )
}
