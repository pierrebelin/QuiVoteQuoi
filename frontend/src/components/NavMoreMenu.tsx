import { useState } from 'react'
import { Menu } from '@base-ui/react/menu'
import { Link, useLocation } from 'react-router'
import { NAV_MORE, isActive } from '../lib/navigation'
import { Icon } from './icons'

const ITEM_CLASS =
  'flex min-h-11 items-center gap-2.5 rounded-[12px] px-3 text-sm font-semibold transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

/**
 * « Plus » de la barre haute : les pages de fond (Comprendre) dans un menu
 * déroulant, comme sous le pouce sur téléphone. Il se ferme
 * à la navigation, à Échap et au clic dehors ; les flèches parcourent ses liens.
 */
export default function NavMoreMenu({ className = '' }: { className?: string }) {
  const location = useLocation()
  const [openedAt, setOpenedAt] = useState<string | null>(null)
  const open = openedAt === location.key
  const active = NAV_MORE.some((link) => isActive(location.pathname, link))

  return (
    <div className={`relative ${className}`}>
      <Menu.Root open={open} onOpenChange={(next) => setOpenedAt(next ? location.key : null)}>
        <Menu.Trigger
          aria-current={active ? 'page' : undefined}
          className={`inline-flex h-10 items-center gap-1.5 rounded-[12px] px-3.5 text-sm font-semibold whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent ${
            active
              ? 'bg-accent-soft text-accent-ink'
              : open
                ? 'bg-surface text-ink'
                : 'text-ink-soft hover:bg-surface hover:text-ink'
          }`}
        >
          Plus
          <Icon name="down" className={`size-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
        </Menu.Trigger>
        <Menu.Portal>
          <Menu.Positioner side="bottom" align="end" sideOffset={6} className="z-30">
            <Menu.Popup className="w-56 rounded border border-line bg-canvas p-1.5 shadow-float outline-none">
              {NAV_MORE.map((link) => {
                const current = isActive(location.pathname, link)
                return (
                  <Menu.LinkItem
                    key={link.to}
                    closeOnClick
                    render={<Link to={link.to} />}
                    aria-current={current ? 'page' : undefined}
                    className={`${ITEM_CLASS} ${
                      current ? 'bg-accent-soft text-accent-ink' : 'text-ink-soft hover:bg-surface hover:text-ink'
                    }`}
                  >
                    <Icon name={link.icon} className="size-4" />
                    {link.label}
                  </Menu.LinkItem>
                )
              })}
            </Menu.Popup>
          </Menu.Positioner>
        </Menu.Portal>
      </Menu.Root>
    </div>
  )
}
