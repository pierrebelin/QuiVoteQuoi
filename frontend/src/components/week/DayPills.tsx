import { capitalized } from '../../lib/entryFlow'
import type { WeekDay, WeekDayState } from '../../types/week'

function weekday(iso: string, weekday: 'short' | 'long') {
  const label = capitalized(new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', { weekday, day: 'numeric' }))
  // « Lun 9 » → « Lun. 9 » : l'abréviation garde son point.
  return weekday === 'short' ? label.replace(/^(\p{L}+)\.? /u, '$1. ') : label
}

/** Jour sans vote : un signe sur téléphone, sa légende à partir de 768 px (RM-06). */
const EMPTY: Record<Exclude<WeekDayState, 'voted'>, { mark: string; caption: string }> = {
  no_public_vote: { mark: '—', caption: 'aucun vote public' },
  not_yet_published: { mark: '…', caption: 'pas encore publié' },
  upcoming: { mark: '…', caption: 'à venir' },
}

/**
 * Une pastille par jour de la semaine, avec son nombre de votes. Un jour sans
 * vote n'est pas cliquable. Une pastille active se désactive en la reprenant :
 * sans jour choisi, la semaine entière est montrée. Sans `onSelect`, les
 * pastilles ne filtrent rien et ne font que montrer la semaine (accueil).
 * `compact` garde le format téléphone à toutes les largeurs (colonne étroite).
 */
export default function DayPills({
  days,
  selected,
  onSelect,
  compact = false,
}: {
  days: WeekDay[]
  selected?: string | null
  onSelect?: (date: string | null) => void
  compact?: boolean
}) {
  const list = (
    <ul className={`flex gap-[5px] ${compact ? '' : 'md:gap-2'}`} aria-label={onSelect ? 'Filtrer par jour' : 'Votes par jour'}>
      {days.map((day) => {
        const active = day.date === selected
        const enabled = day.state === 'voted'
        const empty = day.state === 'voted' ? null : EMPTY[day.state]
        const label = `${weekday(day.date, 'long')} : ${empty ? empty.caption : `${day.votes} vote${day.votes > 1 ? 's' : ''}`}`
        const pill = `flex min-h-[52px] w-full flex-col items-center justify-center rounded-[14px] px-0.5 py-1 whitespace-nowrap ${
          compact ? '' : 'md:min-h-14 md:items-start md:gap-0.5 md:px-4 md:py-2.5'
        } ${
          active ? 'bg-ink text-surface' : 'bg-surface text-ink ring-1 ring-line'
        }`
        const content = compact ? (
          <>
            <span className="text-xs opacity-85">{weekday(day.date, 'short')}</span>
            <span className="figure text-[15px] leading-5">{empty ? empty.mark : day.votes}</span>
          </>
        ) : (
          <>
            <span className="text-xs opacity-85 md:hidden">{weekday(day.date, 'short')}</span>
            <span className="hidden text-sm md:inline">{weekday(day.date, 'long')}</span>
            <span className="flex items-baseline gap-[5px]">
              <span className="figure text-[15px] leading-5 md:text-xl">{empty ? empty.mark : day.votes}</span>
              <span className={`hidden text-[12.5px] md:inline ${active ? '' : 'text-ink-soft'}`}>
                {empty ? empty.caption : `vote${day.votes > 1 ? 's' : ''}`}
              </span>
            </span>
          </>
        )
        return (
          <li key={day.date} className="min-w-0 flex-1">
            {onSelect ? (
              <button
                type="button"
                disabled={!enabled}
                aria-pressed={active}
                aria-label={label}
                onClick={() => onSelect(active ? null : day.date)}
                className={`${pill} transition-colors hover:ring-line-strong disabled:cursor-default disabled:hover:ring-line`}
              >
                {content}
              </button>
            ) : (
              <div className={pill}>
                <span className="sr-only">{label}</span>
                <span aria-hidden className="contents">
                  {content}
                </span>
              </div>
            )}
          </li>
        )
      })}
    </ul>
  )
  if (!onSelect) return list
  return (
    <div className="flex flex-col gap-2">
      <p className="text-[13px] text-ink-soft md:text-[13.5px]">
        <span className="md:hidden">Toucher</span>
        <span className="hidden md:inline">Cliquer sur</span> un jour pour n’en voir que les votes.
      </p>
      {list}
    </div>
  )
}
