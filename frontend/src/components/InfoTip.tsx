import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react'
import { Popover } from '@base-ui/react/popover'
import { Icon } from './icons'

/**
 * Infobulles du mobile.
 *
 * Sur téléphone, les explications affichées en toutes lettres sur grand écran
 * repoussaient le contenu hors de vue. Elles passent derrière une tape ; à partir
 * de 768 px, le texte reste écrit dans la page, sans changement.
 *
 * `title` ne convient pas : il n'existe pas au toucher et ne porte pas de lien.
 *
 * L'icône se place à côté du titre que le texte commente : seule au milieu d'une
 * page, elle ne dirait pas de quoi elle parle.
 */

const VIEWPORT_MARGIN = 12
const TRIGGER_GAP = 6
/** Hauteur de la barre de navigation basse, que la bulle ne doit pas recouvrir. */
const BOTTOM_NAV_HEIGHT = 72

/**
 * Arrête un clic venu d'un portail : React le propage quand même à l'arbre
 * d'origine, où une carte-lien ouvrirait sa page.
 */
function stop(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
}

export function InfoTip({
  label,
  children,
  className = '',
}: {
  /** Nom du bouton pour un lecteur d'écran, et de la bulle ouverte. */
  label: string
  children: ReactNode
  className?: string
}) {
  const [open, setOpen] = useState(false)
  const popupRef = useRef<HTMLDivElement>(null)

  // La bulle prend toute la largeur de l'écran, marges déduites : elle ne suit
  // pas son déclencheur au défilement de la page, elle se ferme. Défiler dans la
  // bulle ne la ferme pas.
  useEffect(() => {
    if (!open) return
    const onScroll = (event: Event) => {
      if (popupRef.current?.contains(event.target as Node)) return
      setOpen(false)
    }
    window.addEventListener('scroll', onScroll, true)
    return () => window.removeEventListener('scroll', onScroll, true)
  }, [open])

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger
        aria-label={label}
        onClick={stop}
        // Icône de 18 px, cible de 44 px : les marges négatives évitent que la
        // cible agrandisse la ligne de titre qui la porte.
        className={`-my-3 inline-flex size-11 shrink-0 items-center justify-center align-middle text-ink-faint hover:text-ink-soft ${className}`}
      >
        <Icon name="info" className="size-[18px]" />
      </Popover.Trigger>
      <Popover.Portal>
        {/* Portail : un parent en `overflow-hidden` couperait la bulle. Le fond
            transparent reçoit la tape qui ferme : sans lui, elle activerait aussi
            le lien situé dessous, et fermer une bulle ouvrirait une autre page.
            Il laisse passer le défilement, qui ferme la bulle à son tour. */}
        <Popover.Backdrop className="fixed inset-0 z-[59]" onClick={stop} />
        <Popover.Positioner
          positionMethod="fixed"
          side="bottom"
          sideOffset={TRIGGER_GAP}
          collisionPadding={{
            top: VIEWPORT_MARGIN,
            right: VIEWPORT_MARGIN,
            bottom: BOTTOM_NAV_HEIGHT,
            left: VIEWPORT_MARGIN,
          }}
          className="z-[60]"
        >
          <Popover.Popup
            ref={popupRef}
            aria-label={label}
            initialFocus={popupRef}
            onClick={(event) => event.stopPropagation()}
            style={{ width: `calc(100vw - ${2 * VIEWPORT_MARGIN}px)` }}
            className="max-h-[60vh] space-y-1.5 overflow-y-auto overscroll-contain rounded-xl border border-line bg-surface px-3.5 py-3 text-[13px] leading-relaxed font-normal tracking-normal text-ink-soft normal-case shadow-card-hover outline-none"
          >
            {children}
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
