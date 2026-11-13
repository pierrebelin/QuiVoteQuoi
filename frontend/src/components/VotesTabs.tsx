import { Link } from 'react-router'
import { FOCUS } from './SheetControls'

/** Onglets de la rubrique Votes : la semaine, puis la liste complète (CU-05). */
export type VotesTab = 'semaine' | 'liste'

const TABS: { value: VotesTab; label: string; to: string }[] = [
  { value: 'semaine', label: 'Cette semaine', to: '/semaine' },
  { value: 'liste', label: 'Tous les votes', to: '/scrutins' },
]

export default function VotesTabs({ current }: { current: VotesTab }) {
  return (
    <nav aria-label="Rubrique Votes" className="mb-4 grid w-full grid-cols-2 gap-1 rounded bg-surface-soft p-[3px] md:w-80">
      {TABS.map((tab) => {
        const on = tab.value === current
        return (
          <Link
            key={tab.value}
            to={tab.to}
            aria-current={on ? 'page' : undefined}
            className={`flex min-h-11 items-center justify-center rounded px-2 text-sm whitespace-nowrap transition-colors ${FOCUS} ${
              on ? 'bg-surface font-semibold text-ink shadow-card' : 'font-medium text-ink-soft hover:text-ink'
            }`}
          >
            {tab.label}
          </Link>
        )
      })}
    </nav>
  )
}
