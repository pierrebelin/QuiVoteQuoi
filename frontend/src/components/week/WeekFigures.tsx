import type { WeekTotals } from '../../types/week'

const plural = (count: number, word: string) => `${word}${count > 1 ? 's' : ''}`

/** Les trois grands nombres d'une semaine : votes publics, adoptés, rejetés. */
export default function WeekFigures({ totals }: { totals: WeekTotals }) {
  const stats = [
    { value: totals.votes, label: `${plural(totals.votes, 'vote')} ${plural(totals.votes, 'public')}`, tone: '' },
    { value: totals.adopted, label: plural(totals.adopted, 'adopté'), tone: 'text-yes' },
    { value: totals.rejected, label: plural(totals.rejected, 'rejeté'), tone: 'text-no' },
  ]
  return (
    <div className="grid grid-cols-3 gap-2 md:gap-4">
      {stats.map((stat) => (
        <div key={stat.label}>
          <p className={`figure text-[30px] leading-[34px] tracking-normal md:text-[40px] md:leading-[44px] ${stat.tone}`}>
            {stat.value.toLocaleString('fr-FR')}
          </p>
          <p className="text-[12.5px] leading-[17px] text-ink-soft md:text-[13.5px]">{stat.label}</p>
        </div>
      ))}
    </div>
  )
}
