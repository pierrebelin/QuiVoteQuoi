import type { CSSProperties } from 'react'
import type { WeekDay } from '../../types/week'
import { EMPTY, weekday } from './dayLabels'

/**
 * Les votes de la semaine, un jour par colonne : le nombre, une barre à hauteur
 * du nombre (la plus haute de la semaine pleine, aucune autre échelle) et le
 * jour. Pour l'accueil : les jours ne filtrent rien. Un jour sans vote garde son
 * signe (« — », « … ») et une barre à plat.
 *
 * Le jour est écrit avant le nombre dans le texte, comme dans les pastilles de la
 * page La semaine ; `flex-col-reverse` remet le nombre au-dessus de la barre.
 */
export default function DayBars({ days }: { days: WeekDay[] }) {
  const peak = Math.max(1, ...days.map((day) => (day.state === 'voted' ? day.votes : 0)))
  return (
    <ul aria-label="Votes par jour" className="grid grid-cols-5 items-end gap-2.5 md:gap-3">
      {days.map((day) => {
        const empty = day.state === 'voted' ? null : EMPTY[day.state]
        const label = `${weekday(day.date, 'long')} : ${empty ? empty.caption : `${day.votes} vote${day.votes > 1 ? 's' : ''}`}`
        const share = empty ? 0 : day.votes / peak
        return (
          <li key={day.date} className="flex flex-col-reverse items-center gap-1.5">
            <span className="sr-only">{label}</span>
            <span aria-hidden className="contents">
              <span className="text-xs whitespace-nowrap text-ink-soft">{weekday(day.date, 'short')}</span>
              <i
                style={{ '--share': share } as CSSProperties}
                className="block h-[calc(var(--share)*64px)] min-h-[3px] w-full rounded-md bg-ink-faint/40 md:h-[calc(var(--share)*96px)] xl:h-[calc(var(--share)*80px)]"
              />
              <b className="figure text-[15px] leading-5 tracking-normal">{empty ? empty.mark : day.votes}</b>
            </span>
          </li>
        )
      })}
    </ul>
  )
}
