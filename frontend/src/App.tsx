import { Outlet, Link, NavLink } from 'react-router'
import Logo from './components/Logo'
import MobileNav from './components/MobileNav'
import ThemeToggle from './components/ThemeToggle'
import { GitHubIcon } from './components/icons'
import { FreshnessFooterLine } from './components/Freshness'
import { GITHUB_REPOSITORY_URL } from './lib/repository'

/*
 * Barre haute, à partir de 768 px : six entrées lisibles. « Accueil » est porté
 * par le logo ; les dossiers s'atteignent depuis les scrutins, qui les
 * regroupent par défaut ; les fiches de groupes depuis la page qui compare leurs votes, où
 * l'on va justement chercher qui est qui. « À propos » conserve la promesse,
 * les sources et les règles déplacées de l'ancien accueil.
 *
 * En dessous, la navigation est la barre basse de `MobileNav`.
 */
const NAV = [
  { to: '/scrutins', label: 'Scrutins' },
  { to: '/votes-par-groupe', label: 'Comparer les votes des groupes' },
  { to: '/candidats-2027', label: 'Candidats 2027' },
  { to: '/themes', label: 'Thèmes' },
  { to: '/comprendre', label: 'Comprendre' },
  { to: '/a-propos', label: 'À propos' },
]

export default function App() {
  return (
    <div className="flex min-h-screen flex-col bg-canvas pb-[var(--mobile-nav-h)] text-ink">
      {/*
        En-tête collant : sur des listes de plusieurs milliers de lignes, une
        navigation qu'il faut remonter chercher n'est pas une navigation. Sous
        768 px il se réduit au logo et au nom : la navigation est en bas.
      */}
      <header className="sticky top-0 z-20 border-b border-line bg-surface/85 backdrop-blur-md">
        <div className="mx-auto flex max-w-7xl items-center justify-between gap-8 px-6 py-2.5">
          <Link
            to="/"
            className="flex items-center gap-2 transition-opacity hover:opacity-70"
          >
            <Logo className="h-9 w-auto shrink-0" />
            <h1 className="text-[15px] font-semibold tracking-tight whitespace-nowrap">
              QuiVote
              <span className="text-accent">Quoi</span>
            </h1>
          </Link>

          <nav className="-mx-1 hidden flex-wrap items-center justify-end gap-0.5 md:flex" aria-label="Navigation principale">
            {NAV.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  `rounded-lg px-2.5 py-1.5 text-[13px] font-medium whitespace-nowrap transition-colors ${
                    isActive
                      ? 'bg-surface-soft text-ink'
                      : 'text-ink-soft hover:bg-surface-soft hover:text-ink'
                  }`
                }
              >
                {item.label}
              </NavLink>
            ))}
            <span className="mx-1 h-4 w-px bg-line" aria-hidden />
            <a
              href={GITHUB_REPOSITORY_URL}
              target="_blank"
              rel="noreferrer"
              aria-label="Consulter le code source sur GitHub"
              title="Code source sur GitHub"
              className="rounded-lg p-1.5 text-ink-soft transition-colors hover:bg-surface-soft hover:text-ink"
            >
              <GitHubIcon className="h-4 w-4" />
            </a>
            <ThemeToggle />
          </nav>
        </div>
      </header>

      <main className="mx-auto w-full max-w-7xl flex-1 px-6 py-8">
        <Outlet />
      </main>

      <footer className="mt-12 border-t border-line px-6 py-6">
        <div className="mx-auto flex max-w-7xl flex-wrap items-baseline gap-x-5 gap-y-2 text-xs">
          <span className="font-medium text-ink-soft">
            Transparence des votes parlementaires
          </span>
          <span className="text-ink-faint">
            Données publiques de l'Assemblée nationale, reprises sans sélection.
          </span>
          <FreshnessFooterLine />
        </div>
      </footer>

      <MobileNav />
    </div>
  )
}
