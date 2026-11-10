import { useState, type ReactNode } from 'react'
import { Link, NavLink, useLocation } from 'react-router'
import { GITHUB_REPOSITORY_URL } from '../lib/repository'
import { BottomSheet } from './BottomSheet'
import { GitHubIcon, Icon, type IconName } from './icons'
import ThemeToggle from './ThemeToggle'

/*
 * Navigation du mobile (sous 768 px) : une barre fixe en bas, sous le pouce.
 * Quatre destinations, puis « Plus » pour le reste. La colonne de « Candidats
 * 2027 » est plus large pour que son libellé tienne sur une ligne.
 */
const TABS: { to: string; label: string; icon: IconName }[] = [
  { to: '/themes', label: 'Thèmes', icon: 'themes' },
  { to: '/scrutins', label: 'Scrutins', icon: 'dossiers' },
  { to: '/votes-par-groupe', label: 'Votes', icon: 'votes' },
  { to: '/candidats-2027', label: 'Candidats 2027', icon: 'candidats' },
]

const MORE_LINKS: { to: string; label: string; icon: IconName }[] = [
  { to: '/dossiers', label: 'Dossiers', icon: 'dossiers' },
  { to: '/comprendre', label: 'Comprendre', icon: 'comprendre' },
  { to: '/a-propos', label: 'À propos', icon: 'apropos' },
]

const TAB_CLASS =
  'flex h-full min-w-0 flex-col items-center justify-center gap-0.5 text-[10.5px] leading-[1.15] tracking-[-0.01em] whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

/** Pastille autour de l'icône : pleine, `bg-ink`, quand l'entrée est active. */
function Pip({ active, children }: { active: boolean; children: ReactNode }) {
  return (
    <span
      className={`flex h-[26px] w-[46px] items-center justify-center rounded-full transition-colors ${
        active ? 'bg-ink text-surface' : ''
      }`}
    >
      {children}
    </span>
  )
}

function isUnder(pathname: string, to: string) {
  return pathname === to || pathname.startsWith(`${to}/`)
}

const ROW_CLASS =
  'flex min-h-[50px] w-full items-center gap-3 border-t border-line px-4 text-[15px] font-medium text-ink transition-colors hover:bg-surface-soft focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

export default function MobileNav() {
  const location = useLocation()
  // Le tiroir est ouvert pour une position précise de l'historique : toute
  // navigation en crée une nouvelle, donc le ferme sans effet à écrire.
  const [openedAt, setOpenedAt] = useState<string | null>(null)
  const moreOpen = openedAt === location.key
  const moreActive = MORE_LINKS.some((link) => isUnder(location.pathname, link.to))

  return (
    <>
      <nav
        aria-label="Navigation principale"
        className="fixed inset-x-0 bottom-0 z-30 border-t border-line bg-surface pb-[env(safe-area-inset-bottom)] md:hidden"
      >
        <div className="grid h-14 grid-cols-[1fr_1fr_.85fr_1.3fr_.85fr]">
          {TABS.map((tab) => (
            <NavLink
              key={tab.to}
              to={tab.to}
              className={({ isActive }) =>
                `${TAB_CLASS} ${isActive ? 'font-semibold text-ink' : 'font-medium text-ink-soft'}`
              }
            >
              {({ isActive }) => (
                <>
                  <Pip active={isActive}>
                    <Icon name={tab.icon} className="size-[22px]" />
                  </Pip>
                  <span>{tab.label}</span>
                </>
              )}
            </NavLink>
          ))}
          <button
            type="button"
            aria-haspopup="dialog"
            aria-expanded={moreOpen}
            aria-current={moreActive ? 'page' : undefined}
            onClick={() => setOpenedAt(location.key)}
            className={`${TAB_CLASS} ${moreActive ? 'font-semibold text-ink' : 'font-medium text-ink-soft'}`}
          >
            <Pip active={moreActive}>
              <Icon name="plus" className="size-[22px]" />
            </Pip>
            <span>Plus</span>
          </button>
        </div>
      </nav>

      <BottomSheet open={moreOpen} onClose={() => setOpenedAt(null)} title="Plus">
        <div className="min-h-0 overflow-y-auto pb-2">
          {MORE_LINKS.map((link) => {
            const current = isUnder(location.pathname, link.to)
            return (
              <Link
                key={link.to}
                to={link.to}
                aria-current={current ? 'page' : undefined}
                className={`${ROW_CLASS} ${current ? 'bg-surface-soft font-semibold' : ''}`}
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
