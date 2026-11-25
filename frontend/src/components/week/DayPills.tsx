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
 * partir de 768 px, une colonne sans fond par jour : le jour en toutes lettres, le
 * nombre, puis une piste horizontale à la même échelle. Le jour est écrit avant le
 * nombre dans le texte ; `flex-col-reverse` remet le nombre au-dessus de la barre
 * sur téléphone, `order-last` met la piste en bas sur ordinateur.
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
      <ul
        className="grid items-end gap-2.5 md:items-start md:gap-6"
        aria-label="Filtrer par jour"
        // Une colonne par jour, week-end compris quand il a eu des votes.
        style={{ gridTemplateColumns: `repeat(${days.length}, minmax(0, 1fr))` }}
      >
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
                className="group flex min-h-11 w-full flex-col-reverse items-center gap-1.5 whitespace-nowrap text-ink transition-colors disabled:cursor-default md:flex-col md:items-stretch md:gap-2 md:text-left"
              >
                <span className={`text-xs md:text-sm md:font-semibold ${active ? 'font-semibold md:text-ink' : 'text-ink-soft'}`}>
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
                <span
                  aria-hidden
                  className="order-last hidden h-2 w-full overflow-hidden rounded-full bg-surface md:block"
                >
                  <i
                    style={{ width: `${share * 100}%` }}
                    className={`block h-full rounded-full transition-colors ${
                      active ? 'bg-ink' : 'bg-ink-faint/70 group-enabled:group-hover:bg-ink-faint'
                    }`}
                  />
                </span>
                <span className="flex items-baseline gap-[5px] md:gap-1.5">
                  <span className="figure text-[15px] leading-5 md:text-[28px] md:leading-none">{empty ? empty.mark : day.votes}</span>
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
