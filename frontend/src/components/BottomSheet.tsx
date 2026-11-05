import { useEffect, useId, useRef, type ReactNode, type RefObject } from 'react'
import { createPortal } from 'react-dom'
import { Icon } from './icons'

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

/**
 * Tiroir qui monte du bas de l'écran (sous 768 px) et reste ancré en bas,
 * centré et borné en largeur, au-delà.
 *
 * Se ferme au toucher du fond, à la touche Échap et par le bouton de l'en-tête.
 * Tant qu'il est ouvert, le défilement de la page est gelé, le focus reste dans
 * le panneau et revient à l'élément qui l'a ouvert à la fermeture. Le parent
 * décide de l'ouverture : changer de route la ferme en cessant de le rendre
 * ouvert, ce composant n'écoute pas l'historique.
 */
export function BottomSheet({
  open,
  onClose,
  title,
  aside,
  initialFocus,
  children,
}: {
  open: boolean
  onClose: () => void
  title: string
  /** Complément d'en-tête, à droite du titre (un compteur, par exemple). */
  aside?: ReactNode
  /** Élément focalisé à l'ouverture ; à défaut, le panneau lui-même. */
  initialFocus?: RefObject<HTMLElement | null>
  children: ReactNode
}) {
  const panelRef = useRef<HTMLDivElement>(null)
  const titleId = useId()

  // Le parent passe souvent une fonction recréée à chaque rendu : la lire par
  // une référence évite de rejouer l'effet (et de reprendre le focus) à chaque
  // frappe dans le champ de recherche.
  const closeRef = useRef(onClose)
  useEffect(() => {
    closeRef.current = onClose
  })

  useEffect(() => {
    if (!open) return
    const panel = panelRef.current
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    ;(initialFocus?.current ?? panel)?.focus({ preventScroll: true })

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        event.preventDefault()
        closeRef.current()
        return
      }
      if (event.key !== 'Tab' || !panel) return
      const items = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
        (item) => item.getClientRects().length > 0,
      )
      if (items.length === 0) {
        event.preventDefault()
        return
      }
      const first = items[0]
      const last = items[items.length - 1]
      const active = document.activeElement
      if (event.shiftKey && (active === first || active === panel)) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && active === last) {
        event.preventDefault()
        first.focus()
      }
    }

    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('keydown', onKeyDown)
      document.body.style.overflow = previousOverflow
      opener?.focus({ preventScroll: true })
    }
  }, [open, initialFocus])

  if (!open) return null

  return createPortal(
    <div className="fixed inset-0 z-50 flex flex-col justify-end">
      <button
        type="button"
        tabIndex={-1}
        aria-hidden
        onClick={onClose}
        className="absolute inset-0 cursor-default bg-black/45 motion-safe:animate-fade-in"
      />
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        className="relative mx-auto flex max-h-[88dvh] min-h-0 w-full flex-col rounded-t-[18px] bg-surface pb-[env(safe-area-inset-bottom)] text-ink shadow-card outline-none motion-safe:animate-sheet-in md:max-w-lg"
      >
        <div className="mx-auto mt-2 mb-1 h-1 w-9 shrink-0 rounded-full bg-line-strong" aria-hidden />
        <div className="flex shrink-0 items-center gap-3 py-1 pr-1 pl-4">
          <h2 id={titleId} className="min-w-0 flex-1 text-base font-semibold">
            {title}
          </h2>
          {aside && <span className="shrink-0 text-[13px] text-ink-soft">{aside}</span>}
          <button
            type="button"
            onClick={onClose}
            aria-label="Fermer"
            className="flex size-11 shrink-0 items-center justify-center rounded-xl text-ink-soft transition-colors hover:bg-surface-soft hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent"
          >
            <Icon name="close" />
          </button>
        </div>
        {children}
      </div>
    </div>,
    document.body,
  )
}
