import { useState } from 'react'
import { Link } from 'react-router'
import type { BudgetExerciseDto, BudgetMissionDto } from '../../types/budget'
import {
  amountParts,
  formatAmount,
  formatLawDate,
  formatMissionShare,
} from '../../lib/budgetFormat'
import { Icon } from '../icons'
import { toSlices, type Slice } from '../../lib/budgetSlices'
import { Donut, Swatch } from './BudgetDonut'

/*
 * Carte « Répartition » : le total du budget général au centre d'un anneau,
 * par sujet QuiVoteQuoi ou par mission. Les plus grosses parts sont nommées,
 * les autres regroupées ; la liste « Par mission » sous la carte les donne
 * toutes.
 */

type Grouping = 'sujet' | 'mission'

interface Subject {
  code: string | null
  label: string
  euros: number
  missions: number
}

/** Missions regroupées par sujet, du plus gros au plus petit. */
function bySubject(missions: BudgetMissionDto[]): Subject[] {
  const subjects = new Map<string, Subject>()
  for (const mission of missions) {
    const key = mission.theme?.code ?? ''
    const subject = subjects.get(key) ?? {
      code: mission.theme?.code ?? null,
      label: mission.theme?.label ?? 'Sans sujet',
      euros: 0,
      missions: 0,
    }
    subject.euros += mission.cp_euros
    subject.missions += 1
    subjects.set(key, subject)
  }
  return [...subjects.values()].sort((a, b) => b.euros - a.euros)
}

function plural(count: number, word: string): string {
  return `${count} ${word}${count > 1 ? 's' : ''}`
}

function sliceLabel<T>(slices: Slice<T>[], total: number): string {
  return slices
    .map((slice) => `${slice.label} ${formatMissionShare(slice.euros, total)}`)
    .join(', ')
}

export function BudgetBreakdown({
  exercise,
  onSelectMission,
}: {
  exercise: BudgetExerciseDto
  onSelectMission: (mission: BudgetMissionDto) => void
}) {
  const [grouping, setGrouping] = useState<Grouping>('sujet')
  const { general_budget: general, year, law } = exercise
  const total = general.cp_euros
  const missions = general.missions
  const refunds = general.refunds_and_rebates_cp_euros
  const programmeCount = missions.reduce((sum, mission) => sum + mission.programmes.length, 0)
  const center = amountParts(total)

  const subjectSlices = toSlices(
    bySubject(missions),
    (subject) => ({
      key: subject.code ?? 'sans-sujet',
      label: subject.label,
      euros: subject.euros,
    }),
    (count) => (count > 1 ? `${count} autres sujets` : '1 autre sujet'),
  )
  const missionSlices = toSlices(
    missions,
    (mission) => ({
      key: mission.name,
      label: mission.name,
      euros: mission.cp_euros,
    }),
    (count) => (count > 1 ? `${count} autres missions` : '1 autre mission'),
  )
  const slices: Slice<Subject | BudgetMissionDto>[] =
    grouping === 'sujet' ? subjectSlices : missionSlices

  return (
    <section
      aria-labelledby="budget-repartition"
      className="flex flex-col gap-3.5 rounded border border-line bg-surface p-[18px] xl:grid xl:grid-cols-[360px_minmax(0,1fr)] xl:gap-x-10 xl:p-6"
    >
      {/* À 1280 px : titre, bascule et anneau à gauche, légende à droite. */}
      <div className="flex flex-col gap-3.5">
        <div className="flex flex-col gap-0.5">
          <h2 id="budget-repartition" className="heading-4 text-ink">
            Répartition
          </h2>
          <p className="text-[13px] leading-[18px] text-ink-soft">
            Crédits de paiement du budget général.
            {grouping === 'sujet' && ' Chaque mission est rattachée à un sujet.'}
          </p>
        </div>

        <div
          role="group"
          aria-label="Regrouper par"
          className="flex gap-1 rounded-full bg-surface-soft p-1"
        >
          {(
            [
              ['mission', 'Par mission'],
              ['sujet', 'Par sujet'],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              type="button"
              aria-pressed={grouping === value}
              onClick={() => setGrouping(value)}
              className={`min-h-10 flex-1 rounded-full text-sm font-medium transition-colors ${
                grouping === value ? 'bg-ink text-canvas' : 'text-ink-soft hover:text-ink'
              }`}
            >
              {label}
            </button>
          ))}
        </div>

        <Donut
          slices={slices}
          size={208}
          label={`Répartition du budget général ${grouping === 'sujet' ? 'par sujet' : 'par mission'} : ${sliceLabel(slices, total)}`}
        >
          <p className="figure text-[30px] leading-[34px] text-ink">
            {center.value}
            <span className="ml-1 text-[0.55em] font-medium">{center.unit}</span>
          </p>
          <p className="max-w-[104px] text-[11.5px] leading-[15px] text-ink-faint">
            budget général {year}
          </p>
        </Donut>
      </div>

      <div className="flex flex-col gap-3.5">
        <ul className="flex flex-col border-t border-line">
          {slices.map((slice) => (
            <li key={slice.key} className="border-b border-line">
              <LegendRow
                slice={slice}
                total={total}
                grouping={grouping}
                onSelectMission={onSelectMission}
              />
            </li>
          ))}
        </ul>

        {refunds !== null && (
          <p className="text-[12.5px] leading-[18px] text-ink-faint">
            Dont {formatAmount(refunds)} de remboursements et dégrèvements d’impôts ;{' '}
            {formatAmount(total - refunds)} hors ceux-ci.
          </p>
        )}
        <p className="border-t border-line pt-2.5 text-[12.5px] leading-[18px] text-ink-soft">
          {missions.length} missions · {programmeCount} programmes · article {law.credits_article}{' '}
          de la loi n° {law.number} du {formatLawDate(law.date)}
        </p>
      </div>
    </section>
  )
}

/**
 * Ligne de légende. Un sujet mène à sa page, une mission ouvre son détail ;
 * la part « autres » n'est qu'une ligne, ses éléments sont dans la liste.
 */
function LegendRow({
  slice,
  total,
  grouping,
  onSelectMission,
}: {
  slice: Slice<Subject | BudgetMissionDto>
  total: number
  grouping: Grouping
  onSelectMission: (mission: BudgetMissionDto) => void
}) {
  const grouped = slice.items.length > 1
  const single = slice.items[0]
  const subject = grouping === 'sujet' && !grouped ? (single as Subject) : null
  const mission = grouping === 'mission' && !grouped ? (single as BudgetMissionDto) : null

  const missionCount =
    grouping === 'sujet'
      ? (slice.items as Subject[]).reduce((sum, item) => sum + item.missions, 0)
      : null
  const subtitle =
    missionCount !== null
      ? plural(missionCount, 'mission')
      : grouped
        ? 'détail dans la liste ci-dessous'
        : null

  const body = (
    <>
      <Swatch colorIndex={slice.colorIndex} />
      <span className="min-w-0 flex-1">
        <span className="block text-sm leading-[19px] text-ink">{slice.label}</span>
        {subtitle && <span className="block text-xs leading-4 text-ink-faint">{subtitle}</span>}
      </span>
      <span className="flex shrink-0 flex-col items-end">
        <span className="text-sm leading-[19px] font-semibold whitespace-nowrap text-ink">
          {formatAmount(slice.euros)}
        </span>
        <span className="text-xs leading-4 whitespace-nowrap text-ink-faint">
          {formatMissionShare(slice.euros, total)}
        </span>
      </span>
    </>
  )
  const rowClass = 'flex min-h-11 w-full items-center gap-2.5 py-2 text-left'

  if (subject?.code) {
    return (
      <Link to={`/themes/${subject.code}`} className={`${rowClass} hover:bg-surface-soft`}>
        {body}
        <Icon name="chevron" className="size-4 shrink-0 text-ink-faint" />
      </Link>
    )
  }
  if (mission) {
    return (
      <button
        type="button"
        onClick={() => onSelectMission(mission)}
        className={`${rowClass} hover:bg-surface-soft`}
      >
        {body}
        <Icon name="chevron" className="size-4 shrink-0 text-ink-faint" />
      </button>
    )
  }
  return <div className={rowClass}>{body}</div>
}
