import { Outlet, Link, NavLink } from 'react-router'
import Logo from './components/Logo'
import ThemeToggle from './components/ThemeToggle'
import { FreshnessBanner, FreshnessFooterLine } from './components/Freshness'
import { GITHUB_REPOSITORY_URL } from './lib/repository'

/*
 * Cinq entrées lisibles. « Accueil » est porté par le logo ; les scrutins
 * s'atteignent depuis les dossiers, dont ils sont le détail ; les fiches de
 * groupes depuis la page qui compare leurs votes, où l'on va justement
 * chercher qui est qui. « À propos » conserve la promesse, les sources et les
 * règles déplacées de l'ancien accueil.
 */
const NAV = [
  { to: '/dossiers', label: 'Dossiers', mobileLabel: 'Dossiers', end: false },
  {
    to: '/votes-par-groupe',
    label: 'Comparer les votes des groupes',
    mobileLabel: 'Comparer',
    end: false,
  },
  { to: '/candidats-2027', label: 'Programmes 2027', mobileLabel: '2027', end: false },
  { to: '/themes', label: 'Thèmes', mobileLabel: 'Thèmes', end: false },
  { to: '/comprendre', label: 'Comprendre', mobileLabel: 'Comprendre', end: false },
  { to: '/a-propos', label: 'À propos', mobileLabel: 'À propos', end: false },
]

function GitHubIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
      className={className}
    >
      <path d="M12 2C6.477 2 2 6.477 2 12c0 4.418 2.865 8.166 6.839 9.489.5.092.683-.217.683-.483 0-.237-.009-1.025-.013-1.86-2.782.604-3.369-1.18-3.369-1.18-.455-1.156-1.11-1.464-1.11-1.464-.908-.62.069-.608.069-.608 1.004.07 1.532 1.03 1.532 1.03.892 1.529 2.341 1.087 2.91.831.091-.646.349-1.087.636-1.337-2.22-.253-4.555-1.11-4.555-4.944 0-1.093.39-1.987 1.029-2.687-.103-.253-.446-1.271.098-2.65 0 0 .84-.269 2.75 1.026A9.564 9.564 0 0 1 12 6.756c.85.004 1.706.115 2.504.337 1.909-1.295 2.748-1.026 2.748-1.026.546 1.379.203 2.397.1 2.65.64.7 1.028 1.594 1.028 2.687 0 3.843-2.339 4.688-4.566 4.936.359.31.678.92.678 1.854 0 1.34-.012 2.421-.012 2.75 0 .268.18.58.688.482A10.001 10.001 0 0 0 22 12c0-5.523-4.477-10-10-10Z" />
    </svg>
  )
}

export default function App() {
  return (
    <div className="flex min-h-screen flex-col bg-canvas text-ink">
      {/*
        En-tête collant : sur des listes de plusieurs milliers de lignes, une
        navigation qu'il faut remonter chercher n'est pas une navigation.
      */}
      <header className="sticky top-0 z-20 border-b border-line bg-surface/85 backdrop-blur-md">
        {/* Sur petit écran le titre et la navigation s'empilent : côte à côte,
            la nav se replie sur trois lignes contre le logo. */}
        <div className="mx-auto flex max-w-7xl flex-col gap-2 px-4 py-2.5 sm:flex-row sm:items-center sm:justify-between sm:gap-8 sm:px-6">
          <div className="flex w-full items-center justify-between sm:w-auto">
            <Link
              to="/"
              className="flex items-center gap-2 transition-opacity hover:opacity-70"
            >
              <Logo className="h-9 w-auto shrink-0" />
              <h1 className="text-[15px] font-semibold tracking-tight">
                QuiVote
                <span className="text-accent">Quoi</span>
              </h1>
            </Link>
            <span className="sm:hidden">
              <ThemeToggle />
            </span>
          </div>

          <nav className="-mx-1 flex flex-wrap items-center gap-0.5 sm:justify-end" aria-label="Navigation principale">
            {NAV.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) =>
                  `rounded-lg px-2.5 py-1.5 text-[13px] font-medium whitespace-nowrap transition-colors ${
                    isActive
                      ? 'bg-surface-soft text-ink'
                      : 'text-ink-soft hover:bg-surface-soft hover:text-ink'
                  }`
                }
              >
                <span className="sm:hidden">{item.mobileLabel}</span>
                <span className="hidden sm:inline">{item.label}</span>
              </NavLink>
            ))}
            <span className="mx-1 hidden h-4 w-px bg-line sm:block" aria-hidden />
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
            <span className="hidden sm:block">
              <ThemeToggle />
            </span>
          </nav>
        </div>
      </header>

      <FreshnessBanner />

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
    </div>
  )
}
