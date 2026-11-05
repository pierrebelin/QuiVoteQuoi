import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from 'react'
import { createPortal } from 'react-dom'
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

function useBubble() {
  const [open, setOpen] = useState(false)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const bubbleRef = useRef<HTMLDivElement>(null)
  const [top, setTop] = useState(0)

  // Avant peinture : la bulle s'ouvre sous le déclencheur, ou au-dessus quand la
  // place manque, sans saut visible.
  useLayoutEffect(() => {
    const trigger = triggerRef.current
    const bubble = bubbleRef.current
    if (!open || !trigger || !bubble) return
    const anchor = trigger.getBoundingClientRect()
    const below = anchor.bottom + TRIGGER_GAP
    const fits =
      below + bubble.offsetHeight <= window.innerHeight - BOTTOM_NAV_HEIGHT
    setTop(
      fits
        ? below
        : Math.max(VIEWPORT_MARGIN, anchor.top - TRIGGER_GAP - bubble.offsetHeight),
    )
    bubble.focus({ preventScroll: true })
  }, [open])

  useEffect(() => {
    if (!open) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      setOpen(false)
      triggerRef.current?.focus()
    }
    // La bulle est placée par rapport à l'écran : au défilement de la page, elle
    // ne suivrait plus son déclencheur. Défiler dans la bulle ne la ferme pas.
    const onScroll = (event: Event) => {
      if (bubbleRef.current?.contains(event.target as Node)) return
      setOpen(false)
    }
    document.addEventListener('keydown', onKeyDown)
    window.addEventListener('scroll', onScroll, true)
    return () => {
      document.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('scroll', onScroll, true)
    }
  }, [open])

  return { open, setOpen, triggerRef, bubbleRef, top }
}

function Bubble({
  id,
  label,
  top,
  bubbleRef,
  onClose,
  children,
}: {
  id: string
  label: string
  top: number
  bubbleRef: RefObject<HTMLDivElement | null>
  onClose: () => void
  children: ReactNode
}) {
  // Portail : un parent en `overflow-hidden` couperait la bulle. Les clics restent
  // arrêtés ici, car React les propage quand même à l'arbre d'origine (carte-lien).
  //
  // Le fond transparent reçoit la tape qui ferme : sans lui, elle activerait aussi
  // le lien situé dessous, et fermer une bulle ouvrirait une autre page. Il laisse
  // passer le défilement, qui ferme la bulle à son tour.
  return createPortal(
    <>
      <div
        aria-hidden
        className="fixed inset-0 z-[59]"
        onClick={(event) => {
          event.preventDefault()
          event.stopPropagation()
          onClose()
        }}
      />
      <div
        id={id}
        ref={bubbleRef}
        role="dialog"
        aria-label={label}
        tabIndex={-1}
        onClick={(event) => event.stopPropagation()}
        style={{ top, left: VIEWPORT_MARGIN, right: VIEWPORT_MARGIN }}
        className="fixed z-[60] max-h-[60vh] space-y-1.5 overflow-y-auto overscroll-contain rounded-xl border border-line bg-surface px-3.5 py-3 text-[13px] leading-relaxed font-normal tracking-normal text-ink-soft normal-case shadow-card-hover outline-none"
      >
        {children}
      </div>
    </>,
    document.body,
  )
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
  const { open, setOpen, triggerRef, bubbleRef, top } = useBubble()
  const id = useId()

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        aria-label={label}
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        onClick={(event) => {
          event.preventDefault()
          event.stopPropagation()
          setOpen(!open)
        }}
        // Icône de 18 px, cible de 44 px : les marges négatives évitent que la
        // cible agrandisse la ligne de titre qui la porte.
        className={`-my-3 inline-flex size-11 shrink-0 items-center justify-center align-middle text-ink-faint hover:text-ink-soft ${className}`}
      >
        <Icon name="info" className="size-[18px]" />
      </button>
      {open && (
        <Bubble id={id} label={label} top={top} bubbleRef={bubbleRef} onClose={() => setOpen(false)}>
          {children}
        </Bubble>
      )}
    </>
  )
}
