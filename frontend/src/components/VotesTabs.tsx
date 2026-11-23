import { Link } from 'react-router'
import { FOCUS } from './SheetControls'

/**
 * Onglets de la rubrique Votes : la semaine, puis la liste complète (CU-05).
 * Soulignés (refonte A) : 2 px d'accent sous l'onglet actif, texte d'encre ;
 * les autres en gris, sans fond.
 */
export type VotesTab = 'semaine' | 'liste'

const TABS: { value: VotesTab; label: string; to: string }[] = [
  { value: 'semaine', label: 'La semaine', to: '/semaine' },
  { value: 'liste', label: 'Tous les votes', to: '/scrutins' },
]

export default function VotesTabs({ current }: { current: VotesTab }) {
  return (
    <nav aria-label="Rubrique Votes" className="mb-1">
      <ul className="flex gap-5 border-b border-line">
        {TABS.map((tab) => {
          const on = tab.value === current
          return (
            <li key={tab.value}>
              <Link
                to={tab.to}
                aria-current={on ? 'page' : undefined}
                className={`-mb-px flex h-11 items-center border-b-2 text-sm font-semibold whitespace-nowrap transition-colors ${FOCUS} ${
                  on ? 'border-accent text-ink' : 'border-transparent text-ink-faint hover:text-ink'
                }`}
              >
                {tab.label}
              </Link>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}
