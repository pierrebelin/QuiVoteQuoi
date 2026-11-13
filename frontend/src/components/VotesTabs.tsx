import { Link } from 'react-router'
import { FOCUS } from './SheetControls'

/** Onglets de la rubrique Votes : la semaine, puis la liste complète (CU-05). */
export type VotesTab = 'semaine' | 'liste'

const TABS: { value: VotesTab; label: string; to: string }[] = [
  { value: 'semaine', label: 'La semaine', to: '/semaine' },
  { value: 'liste', label: 'Tous les votes', to: '/scrutins' },
]

export default function VotesTabs({ current }: { current: VotesTab }) {
  return (
    <nav aria-label="Rubrique Votes" className="mb-1">
      <ul className="flex w-full gap-1 rounded-full bg-surface-soft p-1 md:w-[380px]">
        {TABS.map((tab) => {
          const on = tab.value === current
          return (
            <li key={tab.value} className="flex-1">
              <Link
                to={tab.to}
                aria-current={on ? 'page' : undefined}
                className={`flex h-11 items-center justify-center rounded-full px-5 text-[14.5px] whitespace-nowrap transition-colors ${FOCUS} ${
                  on ? 'bg-surface font-semibold text-ink ring-1 ring-line' : 'font-medium text-ink-soft hover:text-ink'
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
