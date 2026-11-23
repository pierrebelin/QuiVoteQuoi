import { useState } from 'react'
import { Link, useLocation } from 'react-router'
import { NAV_MORE, NAV_TABS, isActive } from '../lib/navigation'
import { GITHUB_REPOSITORY_URL } from '../lib/repository'
import { BottomSheet } from './BottomSheet'
import { GitHubIcon, Icon } from './icons'
import ThemeToggle from './ThemeToggle'

/*
 * Navigation du mobile (sous 768 px) : une barre flottante en bas, sous le
 * pouce, détachée du bord et posée au-dessus du contenu par son ombre.
 * Quatre destinations, puis « Plus » pour le reste (`lib/navigation`). La
 * colonne de « Candidats 2027 » est plus large pour que son libellé tienne sur
 * une ligne. L'entrée active prend tout son bouton en cyan pâle.
 */
const TAB_CLASS =
  'flex h-12 min-w-0 flex-col items-center justify-center gap-[3px] rounded-2xl text-[10.5px] leading-[1.15] tracking-[-0.01em] whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

const tabState = (active: boolean) =>
  active ? 'bg-accent-soft font-semibold text-accent-ink' : 'font-medium text-ink-faint hover:text-ink'

const ROW_CLASS =
  'flex min-h-[50px] w-full items-center gap-3 border-t border-line px-4 text-[15px] font-medium text-ink transition-colors hover:bg-surface focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

export default function MobileNav() {
  const location = useLocation()
  // Le tiroir est ouvert pour une position précise de l'historique : toute
  // navigation en crée une nouvelle, donc le ferme sans effet à écrire.
  const [openedAt, setOpenedAt] = useState<string | null>(null)
  const moreOpen = openedAt === location.key
  const moreActive = NAV_MORE.some((link) => isActive(location.pathname, link))

  return (
    <>
      <nav
        aria-label="Navigation principale"
        className="pointer-events-none fixed inset-x-0 bottom-0 z-30 px-3 pt-2 pb-[calc(14px+env(safe-area-inset-bottom))] md:hidden"
      >
        <div className="pointer-events-auto grid grid-cols-[1fr_1fr_1fr_1.45fr_1fr] rounded-[22px] border border-line bg-canvas p-1.5 shadow-float">
          {NAV_TABS.map((tab) => {
            const active = isActive(location.pathname, tab)
            return (
              <Link
                key={tab.to}
                to={tab.to}
                aria-current={active ? 'page' : undefined}
                className={`${TAB_CLASS} ${tabState(active)}`}
              >
                <Icon name={tab.icon} className="size-5" />
                <span>{tab.label}</span>
              </Link>
            )
          })}
          <button
            type="button"
            aria-haspopup="dialog"
            aria-expanded={moreOpen}
            aria-current={moreActive ? 'page' : undefined}
            onClick={() => setOpenedAt(location.key)}
            className={`${TAB_CLASS} ${tabState(moreActive)}`}
          >
            <Icon name="plus" className="size-5" />
            <span>Plus</span>
          </button>
        </div>
      </nav>

      <BottomSheet open={moreOpen} onClose={() => setOpenedAt(null)} title="Plus">
        <div className="min-h-0 overflow-y-auto pb-2">
          {NAV_MORE.map((link) => {
            const current = isActive(location.pathname, link)
            return (
              <Link
                key={link.to}
                to={link.to}
                aria-current={current ? 'page' : undefined}
                className={`${ROW_CLASS} ${current ? 'bg-accent-soft font-semibold text-accent-ink' : ''}`}
              >
                <Icon name={link.icon} />
                {link.label}
              </Link>
            )
          })}
          <ThemeToggle variant="row" />
          <a
            href={GITHUB_REPOSITORY_URL}
            target="_blank"
            rel="noreferrer"
            className={ROW_CLASS}
          >
            <GitHubIcon className="size-5 shrink-0" />
            Code source sur GitHub
          </a>
        </div>
      </BottomSheet>
    </>
  )
}
