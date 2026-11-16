import { useState } from 'react'
import { Link } from 'react-router'
import type { BudgetExerciseDto, BudgetMissionDto } from '../../types/budget'
import {
  amountParts,
  barWidth,
  formatAmount,
  formatLawDate,
  formatMissionShare,
  formatProgrammeShare,
} from '../../lib/budgetFormat'
import { Icon } from '../icons'
import { sliceColor, toSlices, type Slice } from '../../lib/budgetSlices'
import { NATURES, natureParts, natureSlices, type NaturePart } from '../../lib/budgetNature'
import { Donut, Swatch } from './BudgetDonut'
import { AmountBar } from './BudgetMission'

/*
 * Carte « Répartition » : le total du budget général au centre d'un anneau,
 * par sujet QuiVoteQuoi, par mission ou par nature de dépense. Les plus
 * grosses parts sont nommées, les autres regroupées ; la liste « Par mission »
 * sous la carte les donne toutes. Par nature, les remboursements d'impôts
 * sont une part à eux (décision du 16/11/2026), et le personnel et les
 * interventions se déplient en catégories.
 */

type Grouping = 'sujet' | 'mission' | 'nature'

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

const GROUPING_NAMES: Record<Grouping, string> = {
  sujet: 'par sujet',
  mission: 'par mission',
  nature: 'par nature de dépense',
}

/** Où la nomenclature range les remboursements, tirés des données de la mission. */
function refundsNote(mission: BudgetMissionDto): string {
  const parts = natureParts(mission.nature)
  const placed = parts.map(
    (part) => `${NATURES[part.key].partitive} (${formatAmount(part.euros)})`,
  )
  const listed =
    placed.length > 1 ? `${placed.slice(0, -1).join(', ')} et ${placed[placed.length - 1]}` : placed[0]
  return listed
    ? `Les remboursements d’impôts sont comptés à part : dans la nomenclature, ce sont ${listed}.`
    : 'Les remboursements d’impôts sont comptés à part.'
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
  // Un exercice importé avant le détail par action n'a pas de nature : pas de bascule.
  const hasNature = general.nature_without_refunds.length > 0
  const parts = natureParts(general.nature_without_refunds, refunds)
  const slices: Slice<Subject | BudgetMissionDto | NaturePart>[] =
    grouping === 'sujet' ? subjectSlices : grouping === 'mission' ? missionSlices : natureSlices(parts)
  const refundsMission = missions.find(
    (mission) => mission.name === 'Remboursements et dégrèvements',
  )

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
            {grouping === 'nature'
              ? 'Crédits de paiement du budget général, selon ce qu’ils paient.'
              : 'Crédits de paiement du budget général.'}
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
              ['nature', 'Par nature'],
            ] as const
          )
            .filter(([value]) => value !== 'nature' || hasNature)
            .map(([value, label]) => (
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
          label={`Répartition du budget général ${GROUPING_NAMES[grouping]} : ${sliceLabel(slices, total)}`}
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
              {grouping === 'nature' ? (
                <NatureRow part={slice.items[0] as NaturePart} total={total} />
              ) : (
                <LegendRow
                  slice={slice as Slice<Subject | BudgetMissionDto>}
                  total={total}
                  grouping={grouping}
                  onSelectMission={onSelectMission}
                />
              )}
            </li>
          ))}
        </ul>

        {grouping === 'nature' && (
          <p className="text-[12.5px] leading-[18px] text-ink-faint">
            Nature = « titre » de la nomenclature budgétaire (LOLF, art. 5).
            {refundsMission && ` ${refundsNote(refundsMission)}`}
          </p>
        )}
        {grouping !== 'nature' && refunds !== null && (
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

/** Catégories détaillées : celles qui disent à qui va l'argent, ou de quoi il se compose. */
const CATEGORY_HEADINGS: Partial<Record<NaturePart['key'], string>> = {
  interventions: 'À qui va l’argent',
  personnel: 'De quoi se compose la dépense',
}

/**
 * Ligne d'une nature. Le personnel et les interventions se déplient en
 * catégories ; leur barre se lit dans la nature, pas dans le total.
 */
function NatureRow({ part, total }: { part: NaturePart; total: number }) {
  const [open, setOpen] = useState(false)
  const nature = NATURES[part.key]
  const heading = CATEGORY_HEADINGS[part.key]
  const categories = part.natures
    .flatMap((item) => item.categories)
    .filter((category) => category.cp_euros > 0)
    .sort((a, b) => b.cp_euros - a.cp_euros)
  const largest = Math.max(0, ...categories.map((category) => category.cp_euros))
  const expandable = heading !== undefined && categories.length > 1

  const body = (
    <>
      <Swatch colorIndex={nature.colorIndex} />
      <span className="min-w-0 flex-1">
        <span className="block text-sm leading-[19px] font-semibold text-ink">{nature.label}</span>
        <span className="block text-xs leading-4 text-ink-faint">{nature.hint}</span>
      </span>
      <span className="flex shrink-0 flex-col items-end">
        <span className="text-sm leading-[19px] font-semibold whitespace-nowrap text-ink">
          {formatAmount(part.euros)}
        </span>
        <span className="text-xs leading-4 whitespace-nowrap text-ink-faint">
          {formatMissionShare(part.euros, total)}
        </span>
      </span>
    </>
  )
  const rowClass = 'flex min-h-11 w-full items-center gap-2.5 py-2 text-left'

  if (!expandable) {
    return (
      <div className={rowClass}>
        {body}
        <span aria-hidden="true" className="size-4 shrink-0" />
      </div>
    )
  }
  return (
    <>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className={`${rowClass} hover:bg-surface-soft`}
      >
        {body}
        <Icon
          name="chevron"
          className={`size-4 shrink-0 text-ink-faint transition-transform ${open ? '-rotate-90' : 'rotate-90'}`}
        />
      </button>
      {open && (
        <div className="mb-2 ml-5 border-l-2 border-line py-1 pl-3">
          <p className="pt-0.5 text-xs leading-4 text-ink-faint">{heading}</p>
          <ul>
            {categories.map((category) => (
              <li key={category.code} className="flex flex-col gap-1.5 py-[7px]">
                <span className="flex justify-between gap-2.5 text-[13px] leading-[18px]">
                  <span className="text-ink-soft">{category.label}</span>
                  <span className="font-semibold whitespace-nowrap text-ink">
                    {formatAmount(category.cp_euros)}
                  </span>
                </span>
                <span className="flex items-center gap-2">
                  <span className="flex-1">
                    <AmountBar
                      width={barWidth(category.cp_euros, largest)}
                      color={sliceColor(nature.colorIndex)}
                    />
                  </span>
                  <span className="w-11 text-right text-xs text-ink-faint">
                    {formatProgrammeShare(category.cp_euros, part.euros)}
                  </span>
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </>
  )
}
