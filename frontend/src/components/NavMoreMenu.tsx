import { useEffect, useId, useRef, useState } from 'react'
import { Link, useLocation } from 'react-router'
import { NAV_MORE, isActive } from '../lib/navigation'
import { Icon } from './icons'

const ITEM_CLASS =
  'flex min-h-10 items-center gap-2.5 rounded-lg px-3 text-[13px] font-medium transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

/**
 * « Plus » de la barre haute : les pages de fond (Comprendre, À
 * propos) dans un menu déroulant, comme sous le pouce sur téléphone. Il se ferme
 * à la navigation, à Échap et au clic dehors.
 */
export default function NavMoreMenu({ className = '' }: { className?: string }) {
  const location = useLocation()
  const [openedAt, setOpenedAt] = useState<string | null>(null)
  const open = openedAt === location.key
  const active = NAV_MORE.some((link) => isActive(location.pathname, link))
  const menuId = useId()
  const root = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    function onPointer(event: PointerEvent) {
      if (!root.current?.contains(event.target as Node)) setOpenedAt(null)
    }
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape') setOpenedAt(null)
    }
    document.addEventListener('pointerdown', onPointer)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('pointerdown', onPointer)
      document.removeEventListener('keydown', onKey)
    }
  }, [open])

  return (
    <div ref={root} className={`relative ${className}`}>
      <button
        type="button"
        aria-expanded={open}
        aria-controls={menuId}
        aria-current={active ? 'page' : undefined}
        onClick={() => setOpenedAt(open ? null : location.key)}
        className={`inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-[13px] font-medium whitespace-nowrap transition-colors ${
          active || open
            ? 'bg-surface-soft text-ink'
            : 'text-ink-soft hover:bg-surface-soft hover:text-ink'
        }`}
      >
        Plus
        <Icon name="down" className={`size-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && (
        <div
          id={menuId}
          className="absolute right-0 top-full z-30 mt-1.5 w-52 rounded-xl border border-line bg-surface p-1.5 shadow-card-hover"
        >
          {NAV_MORE.map((link) => {
            const current = isActive(location.pathname, link)
            return (
              <Link
                key={link.to}
                to={link.to}
                aria-current={current ? 'page' : undefined}
                className={`${ITEM_CLASS} ${
                  current ? 'bg-surface-soft text-ink' : 'text-ink-soft hover:bg-surface-soft hover:text-ink'
                }`}
              >
                <Icon name={link.icon} className="size-4" />
                {link.label}
              </Link>
            )
          })}
        </div>
      )}
    </div>
  )
}
