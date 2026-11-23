import type { CSSProperties } from 'react'
import type { WeekDay } from '../../types/week'
import { EMPTY, weekday } from './dayLabels'

/**
 * Les jours de la semaine, avec leur nombre de votes, comme filtre. Un jour sans
 * vote n'est pas cliquable. Un jour actif se désactive en le reprenant : sans
 * jour choisi, la semaine entière est montrée.
 *
 * Sous 768 px, une colonne par jour : le nombre, une barre à hauteur du nombre
 * (la plus haute de la semaine pleine, aucune autre échelle) et le jour. À
 * partir de 768 px, une pastille par jour, le jour en toutes lettres. Le jour est
 * écrit avant le nombre dans le texte ; `flex-col-reverse` remet le nombre au-dessus
 * de la barre.
 */
export default function DayPills({
  days,
  selected,
  onSelect,
}: {
  days: WeekDay[]
  selected?: string | null
  onSelect: (date: string | null) => void
}) {
  const peak = Math.max(1, ...days.map((day) => (day.state === 'voted' ? day.votes : 0)))
  return (
    <div className="flex flex-col gap-2.5">
      <p className="text-[13px] text-ink-soft md:text-[13.5px]">
        <span className="md:hidden">Toucher</span>
        <span className="hidden md:inline">Cliquer sur</span> un jour pour n’en voir que les votes.
      </p>
      <ul className="grid grid-cols-5 items-end gap-2.5 md:gap-3" aria-label="Filtrer par jour">
        {days.map((day) => {
          const active = day.date === selected
          const enabled = day.state === 'voted'
          const empty = day.state === 'voted' ? null : EMPTY[day.state]
          const label = `${weekday(day.date, 'long')} : ${empty ? empty.caption : `${day.votes} vote${day.votes > 1 ? 's' : ''}`}`
          const share = empty ? 0 : day.votes / peak
          return (
            <li key={day.date} className="min-w-0">
              <button
                type="button"
                disabled={!enabled}
                aria-pressed={active}
                aria-label={label}
                onClick={() => onSelect(active ? null : day.date)}
                className={`group flex min-h-11 w-full flex-col-reverse items-center gap-1.5 whitespace-nowrap transition-colors disabled:cursor-default md:min-h-14 md:flex-col md:items-start md:justify-center md:gap-0.5 md:rounded-[14px] md:px-4 md:py-2.5 ${
                  active ? 'text-ink md:bg-ink md:text-surface' : 'text-ink md:bg-surface md:enabled:hover:bg-surface-soft'
                }`}
              >
                <span
                  className={`text-xs md:text-sm md:opacity-100 ${active ? 'font-semibold' : 'text-ink-soft'} ${
                    active ? 'md:text-surface' : ''
                  }`}
                >
                  <span className="md:hidden">{weekday(day.date, 'short')}</span>
                  <span className="hidden md:inline">{weekday(day.date, 'long')}</span>
                </span>
                <i
                  aria-hidden
                  style={{ '--share': share } as CSSProperties}
                  className={`block h-[calc(var(--share)*64px)] min-h-[3px] w-full rounded-md transition-colors md:hidden ${
                    active ? 'bg-ink' : 'bg-ink-faint/40 group-enabled:group-hover:bg-ink-faint/60'
                  }`}
                />
                <span className="flex items-baseline gap-[5px]">
                  <span className="figure text-[15px] leading-5 md:text-xl">{empty ? empty.mark : day.votes}</span>
                  <span className={`hidden text-[12.5px] md:inline ${active ? '' : 'text-ink-soft'}`}>
                    {empty ? empty.caption : `vote${day.votes > 1 ? 's' : ''}`}
                  </span>
                </span>
              </button>
            </li>
          )
        })}
      </ul>
    </div>
  )
}
