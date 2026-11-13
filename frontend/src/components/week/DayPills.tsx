import { capitalized } from '../../lib/entryFlow'
import type { WeekDay } from '../../types/week'

function weekdayLabel(iso: string) {
  return capitalized(new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', { weekday: 'short', day: 'numeric' }))
}

/**
 * Une pastille par jour de la semaine, avec son nombre de votes. Un jour sans
 * vote n'est pas cliquable. Une pastille active se désactive en la reprenant :
 * sans jour choisi, la semaine entière est montrée.
 */
export default function DayPills({
  days,
  selected,
  onSelect,
}: {
  days: WeekDay[]
  selected: string | null
  onSelect: (date: string | null) => void
}) {
  return (
    <div className="flex flex-wrap gap-1.5" role="group" aria-label="Filtrer par jour">
      {days.map((day) => {
        const active = day.date === selected
        const enabled = day.state === 'voted'
        return (
          <button
            key={day.date}
            type="button"
            disabled={!enabled}
            aria-pressed={active}
            onClick={() => onSelect(active ? null : day.date)}
            className={`rounded-full px-3 py-1 text-[13px] font-medium tabular-nums transition disabled:cursor-default disabled:opacity-50 ${
              active ? 'bg-ink text-surface' : 'bg-surface-soft text-ink ring-1 ring-line hover:ring-line-strong'
            }`}
          >
            {weekdayLabel(day.date)}
            {enabled && <span className="ml-1.5 text-ink-faint">{day.votes}</span>}
          </button>
        )
      })}
    </div>
  )
}
