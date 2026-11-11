import { useState } from 'react'
import { Menu } from '@base-ui/react/menu'
import { Link, useLocation } from 'react-router'
import { NAV_MORE, isActive } from '../lib/navigation'
import { Icon } from './icons'

const ITEM_CLASS =
  'flex min-h-10 items-center gap-2.5 rounded px-3 text-[13px] font-medium transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

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
          className={`inline-flex items-center gap-1 rounded px-2.5 py-1.5 text-[13px] font-medium whitespace-nowrap transition-colors ${
            active || open
              ? 'bg-surface-soft text-ink'
              : 'text-ink-soft hover:bg-surface-soft hover:text-ink'
          }`}
        >
          Plus
          <Icon name="down" className={`size-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
        </Menu.Trigger>
        <Menu.Portal>
          <Menu.Positioner side="bottom" align="end" sideOffset={6} className="z-30">
            <Menu.Popup className="w-52 rounded border border-line bg-surface p-1.5 shadow-card-hover outline-none">
              {NAV_MORE.map((link) => {
                const current = isActive(location.pathname, link)
                return (
                  <Menu.LinkItem
                    key={link.to}
                    closeOnClick
                    render={<Link to={link.to} />}
                    aria-current={current ? 'page' : undefined}
                    className={`${ITEM_CLASS} ${
                      current ? 'bg-surface-soft text-ink' : 'text-ink-soft hover:bg-surface-soft hover:text-ink'
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
