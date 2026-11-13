import { useEffect, useRef } from 'react'
import { Outlet, Link, useLocation } from 'react-router'
import Logo from './components/Logo'
import MobileNav from './components/MobileNav'
import NavMoreMenu from './components/NavMoreMenu'
import ThemeToggle from './components/ThemeToggle'
import { GitHubIcon } from './components/icons'
import { FreshnessFooterLine } from './components/Freshness'
import { NAV_TABS, isActive } from './lib/navigation'
import { GITHUB_REPOSITORY_URL } from './lib/repository'

/*
 * Barre haute, à partir de 768 px : les mêmes entrées que la barre basse du
 * téléphone (`lib/navigation`), « Plus » ouvrant les pages de fond. « Accueil »
 * est porté par le logo.
 *
 * En dessous, la navigation est la barre basse de `MobileNav`.
 */
/**
 * Changement de page : le contenu monte de quelques pixels en apparaissant.
 * Le premier rendu n'est pas animé (rien ne change), ni les visites qui ont
 * demandé moins d'animations. Le contenu n'est pas remonté : l'état des
 * pages (filtres, tiroirs) n'est pas touché.
 */
function usePageEntrance(pathname: string) {
  const ref = useRef<HTMLElement>(null)
  // Chemin déjà affiché : l'effet relancé sans navigation (StrictMode, en
  // développement) ne rejoue pas l'entrée.
  const affiche = useRef(pathname)
  useEffect(() => {
    if (affiche.current === pathname) return
    affiche.current = pathname
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
    ref.current?.animate(
      [
        { opacity: 0, transform: 'translateY(8px)' },
        { opacity: 1, transform: 'none' },
      ],
      { duration: 280, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' },
    )
  }, [pathname])
  return ref
}

export default function App() {
  const { pathname } = useLocation()
  const main = usePageEntrance(pathname)
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
            {NAV_TABS.map((item) => {
              const active = isActive(pathname, item)
              return (
                <Link
                  key={item.to}
                  to={item.to}
                  aria-current={active ? 'page' : undefined}
                  className={`rounded px-2.5 py-1.5 text-[13px] font-medium whitespace-nowrap transition-colors ${
                    active
                      ? 'bg-surface-soft text-ink'
                      : 'text-ink-soft hover:bg-surface-soft hover:text-ink'
                  }`}
                >
                  {item.label}
                </Link>
              )
            })}
            <NavMoreMenu />
            <span className="mx-1 h-4 w-px bg-line" aria-hidden />
            <a
              href={GITHUB_REPOSITORY_URL}
              target="_blank"
              rel="noreferrer"
              aria-label="Consulter le code source sur GitHub"
              title="Code source sur GitHub"
              className="rounded p-1.5 text-ink-soft transition-colors hover:bg-surface-soft hover:text-ink"
            >
              <GitHubIcon className="h-4 w-4" />
            </a>
            <ThemeToggle />
          </nav>
        </div>
      </header>

      <main ref={main} className="mx-auto w-full max-w-7xl flex-1 px-6 py-8">
        <Outlet />
      </main>

      {/* Marges sur le conteneur, comme l'en-tête et le contenu : sinon, au-delà
          de 1280 px, le pied de page commence 24 px plus à gauche. */}
      <footer className="mt-12 border-t border-line py-6">
        <div className="mx-auto flex max-w-7xl flex-wrap items-baseline gap-x-5 gap-y-2 px-6 text-xs">
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
