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
      <header className="sticky top-0 z-20 border-b border-line bg-canvas/90 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-7xl items-center justify-between gap-8 pr-4 pl-5 md:h-16 md:px-6">
          <Link
            to="/"
            className="flex items-center gap-2.5 transition-opacity hover:opacity-70"
          >
            <Logo className="h-[30px] w-auto shrink-0" />
            <h1 className="text-base font-semibold tracking-[-0.01em] whitespace-nowrap">
              QuiVote
              <span className="text-accent">Quoi</span>
            </h1>
          </Link>

          <nav className="hidden flex-wrap items-center justify-end gap-0.5 md:flex" aria-label="Navigation principale">
            {NAV_TABS.map((item) => {
              const active = isActive(pathname, item)
              return (
                <Link
                  key={item.to}
                  to={item.to}
                  aria-current={active ? 'page' : undefined}
                  className={`flex h-10 items-center rounded-[12px] px-3.5 text-sm font-semibold whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent ${
                    active
                      ? 'bg-accent-soft text-accent-ink'
                      : 'text-ink-soft hover:bg-surface hover:text-ink'
                  }`}
                >
                  {item.label}
                </Link>
              )
            })}
            <NavMoreMenu />
            <a
              href={GITHUB_REPOSITORY_URL}
              target="_blank"
              rel="noreferrer"
              aria-label="Consulter le code source sur GitHub"
              title="Code source sur GitHub"
              className="ml-1 inline-flex size-11 items-center justify-center rounded-[14px] text-ink-soft transition-colors hover:bg-surface hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent"
            >
              <GitHubIcon className="size-5" />
            </a>
            <ThemeToggle />
          </nav>
        </div>
      </header>

      <main ref={main} className="mx-auto w-full max-w-7xl flex-1 px-5 py-8 md:px-6">
        <Outlet />
      </main>

      {/* Marges sur le conteneur, comme l'en-tête et le contenu : sinon, au-delà
          de 1280 px, le pied de page commence 24 px plus à gauche. */}
      {/* Mobile : trois lignes empilées. Ordinateur : l'identité à gauche, la
          fraîcheur des données à droite. */}
      <footer className="mt-12 border-t border-line">
        <div className="mx-auto flex max-w-7xl flex-col gap-1.5 px-5 pt-7 pb-5 text-[13px] leading-snug text-ink-faint md:flex-row md:items-start md:justify-between md:gap-6 md:px-6 md:pt-6 md:pb-8">
          <span className="flex flex-col gap-1.5 md:gap-1">
            <span className="font-semibold text-ink-soft">
              Transparence des votes parlementaires
            </span>
            <span>
              Données publiques de l'Assemblée nationale, reprises sans sélection.
            </span>
          </span>
          <span className="md:max-w-[50%] md:text-right">
            <FreshnessFooterLine />
          </span>
        </div>
      </footer>

      <MobileNav />
    </div>
  )
}
