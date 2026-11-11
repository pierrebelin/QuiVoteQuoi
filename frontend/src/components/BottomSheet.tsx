import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react'
import { Drawer } from '@base-ui/react/drawer'
import { useMediaQuery } from '../lib/useMediaQuery'
import { Icon } from './icons'

/*
 * Le glisser vient de Base UI : pendant le geste, il place le panneau en
 * `transform` en ligne ; au relâcher, il garde le déplacement dans
 * `--drawer-swipe-movement-y` (ou `-x`) et la transition ramène le panneau
 * (retour élastique) ou l'emporte hors de l'écran (`data-swipe-dismiss`). Les
 * autres fermetures restent immédiates, comme avant. Le panneau se déplace par
 * `transform` (et non `translate`) pour ne pas cumuler les deux pendant le geste.
 */
const SWIPE_DOWN =
  '[transform:translateY(var(--drawer-swipe-movement-y))] data-swipe-dismiss:data-ending-style:[transform:translateY(100%)]'
const SWIPE_RIGHT =
  'lg:[transform:translateX(var(--drawer-swipe-movement-x))] lg:data-swipe-dismiss:data-ending-style:[transform:translateX(100%)]'
const SWIPE_RELEASE =
  'motion-safe:transition-transform motion-safe:duration-[calc(var(--drawer-swipe-strength,1)*300ms)] motion-safe:ease-[cubic-bezier(0.32,0.72,0,1)] data-swiping:transition-none data-swiping:select-none'

/**
 * Tiroir qui monte du bas de l'écran (sous 768 px) et reste ancré en bas,
 * centré et borné en largeur, au-delà.
 *
 * Se ferme au toucher du fond, à la touche Échap, par le bouton de l'en-tête et
 * en le faisant glisser vers le bas (au-delà de la moitié de sa hauteur, ou
 * d'un geste vif ; en deçà, il revient en place). Le contenu qui défile garde
 * son défilement : le glisser ne ferme que depuis le haut de la liste. Tant
 * qu'il est ouvert, le défilement de la page est gelé, le focus reste dans le
 * panneau et revient à l'élément qui l'a ouvert à la fermeture. Le parent
 * décide de l'ouverture : changer de route la ferme en cessant de le rendre
 * ouvert, ce composant n'écoute pas l'historique.
 *
 * Avec `sideOnWide`, à partir de 1024 px le tiroir devient un panneau latéral
 * de 420 px, plein hauteur, collé au bord droit : à cette largeur, un tiroir
 * bas laisserait le contenu flotter au milieu d'un écran vide. Il se ferme
 * alors en glissant vers la droite. Les autres appels gardent le tiroir bas à
 * toutes les largeurs.
 *
 * Avec `footer`, les enfants défilent dans le panneau et le pied reste visible
 * dessous (le bouton qui referme le tiroir ne se perd pas dans une longue
 * liste). Sans `footer`, les enfants gèrent eux-mêmes leur défilement.
 */
export function BottomSheet({
  open,
  onClose,
  title,
  aside,
  initialFocus,
  footer,
  sideOnWide = false,
  children,
}: {
  open: boolean
  onClose: () => void
  title: string
  /** Complément d'en-tête, à droite du titre (un compteur, par exemple). */
  aside?: ReactNode
  /** Élément focalisé à l'ouverture ; à défaut, le panneau lui-même. */
  initialFocus?: RefObject<HTMLElement | null>
  /** Pied fixe sous le contenu défilant (boutons d'action). */
  footer?: ReactNode
  /** Panneau latéral à droite à partir de 1024 px (tiroir bas en dessous). */
  sideOnWide?: boolean
  children: ReactNode
}) {
  const panelRef = useRef<HTMLDivElement>(null)
  const side = useMediaQuery('(min-width: 1024px)') && sideOnWide

  // Base UI traite un tiroir monté déjà ouvert comme un état de page (pas de
  // focus déplacé dedans) : certains parents le montent ouvert (la répartition
  // d'un groupe). Partir fermé et s'ouvrir au rendu suivant le fait passer par
  // une vraie ouverture.
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])

  return (
    <Drawer.Root
      open={open && mounted}
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
      swipeDirection={side ? 'right' : 'down'}
    >
      <Drawer.Portal>
        <Drawer.Backdrop className="fixed inset-0 z-50 bg-black/45 opacity-[calc(1-var(--drawer-swipe-progress,0))] motion-safe:animate-fade-in" />
        <Drawer.Viewport
          className={`fixed inset-0 z-50 flex flex-col justify-end ${sideOnWide ? 'lg:items-end lg:justify-start' : ''}`}
        >
          <Drawer.Popup
            ref={panelRef}
            initialFocus={initialFocus ?? panelRef}
            className={`relative mx-auto flex max-h-[88dvh] min-h-0 w-full flex-col rounded-t-[18px] bg-surface pb-[env(safe-area-inset-bottom)] text-ink shadow-card outline-none md:max-w-lg ${SWIPE_RELEASE} ${
              sideOnWide
                ? `${side ? SWIPE_RIGHT : SWIPE_DOWN} motion-safe:animate-sheet-in lg:mx-0 lg:h-full lg:max-h-none lg:w-[420px] lg:max-w-[420px] lg:rounded-none lg:motion-safe:animate-fade-in`
                : `${SWIPE_DOWN} motion-safe:animate-sheet-in`
            }`}
          >
            <div
              className={`mx-auto mt-2 mb-1 h-1 w-9 shrink-0 rounded-full bg-line-strong ${sideOnWide ? 'lg:hidden' : ''}`}
              aria-hidden
            />
            <div className="flex shrink-0 items-center gap-3 py-1 pr-1 pl-4">
              <Drawer.Title className="min-w-0 flex-1 text-base font-semibold">{title}</Drawer.Title>
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
            {footer ? (
              <>
                <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 pb-3">
                  {children}
                </div>
                <div className="flex shrink-0 gap-2 border-t border-line px-4 py-2.5">{footer}</div>
              </>
            ) : (
              children
            )}
          </Drawer.Popup>
        </Drawer.Viewport>
      </Drawer.Portal>
    </Drawer.Root>
  )
}
