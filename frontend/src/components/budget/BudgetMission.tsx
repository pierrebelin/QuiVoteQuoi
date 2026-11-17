import { useState } from 'react'
import { Link } from 'react-router'
import type { BudgetMissionDto, BudgetNatureDto, BudgetProgrammeDto } from '../../types/budget'
import {
  amountParts,
  barWidth,
  formatAmount,
  formatMissionShare,
  formatNatureShare,
  formatProgrammeShare,
  accountLabel,
} from '../../lib/budgetFormat'
import { Icon } from '../icons'
import { sliceColor, toSlices } from '../../lib/budgetSlices'
import { NATURES, natureParts } from '../../lib/budgetNature'
import { Donut, Swatch } from './BudgetDonut'

/*
 * Pièces de la page budget : la barre, la ligne d'une mission et son détail
 * (nature de ses crédits, programmes dépliables jusqu'aux actions, sujet). Le
 * détail est le même dans le tiroir du téléphone et dans le panneau de droite
 * à 1280 px.
 */

/** Barre de montant ; `color` reprend la couleur d'une part d'anneau, l'accent sinon. */
export function AmountBar({ width, color }: { width: string; color?: string }) {
  return (
    <span aria-hidden="true" className="block h-2 overflow-hidden rounded-full bg-surface-soft">
      <span
        className="block h-2 rounded-full bg-accent"
        style={color ? { width, backgroundColor: color } : { width }}
      />
    </span>
  )
}

/**
 * Ligne d'une mission. Sous 1280 px, une carte : nom et montant, puis barre et
 * part. À 1280 px, une ligne de tableau (mission, barre, montant, part) : les
 * deux rangées passent en `display: contents` et `order` replace la barre.
 */
export function MissionRow({
  mission,
  total,
  largest,
  selected,
  wide,
  onSelect,
}: {
  mission: BudgetMissionDto
  total: number
  largest: number
  selected: boolean
  wide: boolean
  onSelect: () => void
}) {
  return (
    <li>
      <button
        type="button"
        onClick={onSelect}
        aria-haspopup={wide ? undefined : 'dialog'}
        aria-pressed={wide ? selected : undefined}
        className={`flex w-full flex-col gap-2 rounded-[14px] border px-3.5 py-3 text-left transition-colors xl:grid xl:grid-cols-[minmax(0,300px)_minmax(0,1fr)_88px_56px] xl:items-center xl:gap-4 xl:rounded-xl xl:py-2 ${
          selected
            ? 'border-accent bg-accent-soft'
            : 'border-line bg-surface hover:border-line-strong xl:border-transparent xl:bg-transparent xl:hover:bg-surface-soft'
        }`}
      >
        <span className="flex w-full items-baseline justify-between gap-3 xl:contents">
          <span className="text-[14.5px] leading-5 font-semibold text-ink xl:order-1 xl:text-sm xl:font-medium">
            {mission.name}
          </span>
          <span className="text-[14.5px] leading-5 font-semibold whitespace-nowrap text-ink xl:order-3 xl:text-right xl:text-sm">
            {formatAmount(mission.cp_euros)}
          </span>
        </span>
        <span className="flex w-full items-center gap-2.5 xl:contents">
          <span className="flex-1 xl:order-2">
            <AmountBar width={barWidth(mission.cp_euros, largest)} />
          </span>
          <span className="w-12 text-right text-[12.5px] leading-4 text-ink-faint xl:order-4 xl:w-auto">
            {formatMissionShare(mission.cp_euros, total)}
          </span>
        </span>
      </button>
    </li>
  )
}

/** Ligne d'un compte hors budget général : pas de part, il n'entre pas dans le total. */
export function OffBudgetRow({
  mission,
  selected,
  wide,
  onSelect,
}: {
  mission: BudgetMissionDto
  selected: boolean
  wide: boolean
  onSelect: () => void
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-haspopup={wide ? undefined : 'dialog'}
      aria-pressed={wide ? selected : undefined}
      className={`flex w-full flex-col gap-1.5 rounded-[14px] border px-3.5 py-3 text-left transition-colors ${
        selected ? 'border-accent bg-accent-soft' : 'border-line bg-surface hover:border-line-strong'
      }`}
    >
      <span className="flex w-full items-baseline justify-between gap-3">
        <span className="text-[14.5px] leading-5 font-semibold text-ink">{mission.name}</span>
        <span className="text-[14.5px] leading-5 font-semibold whitespace-nowrap text-ink">
          {formatAmount(mission.cp_euros)}
        </span>
      </span>
      <span className="text-[12.5px] leading-[17px] text-ink-faint">{accountLabel(mission)}</span>
    </button>
  )
}

/**
 * Programmes de la mission et lien vers son sujet. L'anneau nomme les plus
 * gros programmes ; chaque ligne de la liste prend la couleur de sa part.
 */
export function MissionDetail({ mission, year }: { mission: BudgetMissionDto; year: number }) {
  const largest = Math.max(0, ...mission.programmes.map((programme) => programme.cp_euros))
  const byAmount = [...mission.programmes].sort((a, b) => b.cp_euros - a.cp_euros)
  const slices = toSlices(
    byAmount,
    (programme) => ({
      key: String(programme.number),
      label: programme.name,
      euros: programme.cp_euros,
    }),
    (count) => `${count} autre${count > 1 ? 's' : ''} programme${count > 1 ? 's' : ''}`,
  )
  const colorOf = new Map(
    slices.flatMap((slice) => slice.items.map((programme) => [programme.number, slice.colorIndex])),
  )
  const center = amountParts(mission.cp_euros)
  const actionCount = mission.programmes.reduce((sum, programme) => sum + programme.actions.length, 0)
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-4">
        <Donut
          slices={slices}
          size={132}
          label={`Répartition entre les programmes : ${slices
            .map((slice) => `${slice.label} ${formatProgrammeShare(slice.euros, mission.cp_euros)}`)
            .join(', ')}`}
        >
          <p className="figure text-[22px] leading-[26px] text-ink">
            {center.value}
            <span className="ml-1 text-[0.55em] font-medium">{center.unit}</span>
          </p>
          <p className="text-[11.5px] leading-[15px] text-ink-faint">en {year}</p>
        </Donut>
        <p className="text-[13px] leading-[19px] text-ink-soft">
          {mission.account_kind !== 'budget_general'
            ? 'Ce compte est hors budget général : il n’entre pas dans le total.'
            : `Répartition entre ses ${mission.programmes.length} programmes, puis entre leurs ${actionCount} actions.`}
        </p>
      </div>
      <section aria-labelledby="mission-nature" className="flex flex-col gap-2">
        <h3 id="mission-nature" className="text-[14.5px] leading-5 font-semibold text-ink">
          Ce que paient ces crédits
        </h3>
        <NatureBar natures={mission.nature} total={mission.cp_euros} height="h-3" />
      </section>
      <section aria-labelledby="mission-programmes">
        <h3 id="mission-programmes" className="text-[14.5px] leading-5 font-semibold text-ink">
          Programmes
        </h3>
        <ul key={mission.name} className="flex flex-col border-b border-line">
          {byAmount.map((programme) => (
            <ProgrammeItem
              key={programme.number}
              programme={programme}
              missionTotal={mission.cp_euros}
              largest={largest}
              colorIndex={colorOf.get(programme.number) ?? 0}
              initiallyOpen={mission.programmes.length === 1}
            />
          ))}
        </ul>
      </section>
      {mission.theme && (
        <Link
          to={`/themes/${mission.theme.code}`}
          className="flex min-h-[52px] items-center justify-between gap-3 rounded-[14px] bg-accent-soft px-3.5 py-2.5 text-accent-strong"
        >
          <span>
            <span className="block text-[12.5px] leading-[17px] text-ink-soft">Sujet QuiVoteQuoi</span>
            <span className="block text-[14.5px] leading-5 font-semibold">{mission.theme.label}</span>
          </span>
          <Icon name="chevron" />
        </Link>
      )}
    </div>
  )
}

/**
 * Barre empilée de la nature d'une dépense, puis sa légende. Chaque nature
 * garde sa couleur (lib/budgetNature.ts) ; une part à 0 € n'est pas tracée.
 */
function NatureBar({
  natures,
  total,
  height,
}: {
  natures: BudgetNatureDto[]
  total: number
  height: 'h-2' | 'h-3'
}) {
  const parts = natureParts(natures)
  if (parts.length === 0) return null
  const label = parts
    .map((part) => `${NATURES[part.key].label} ${formatNatureShare(part.euros, total)}`)
    .join(', ')
  return (
    <div className="flex flex-col gap-2">
      <span
        role="img"
        aria-label={`Nature de la dépense : ${label}`}
        className="flex w-full gap-0.5 overflow-hidden rounded-full"
      >
        {parts.map((part) => (
          <span
            key={part.key}
            className={`block ${height}`}
            style={{
              width: barWidth(part.euros, total),
              backgroundColor: sliceColor(NATURES[part.key].colorIndex),
            }}
          />
        ))}
      </span>
      <ul aria-hidden="true" className="flex flex-wrap gap-x-3.5 gap-y-1">
        {parts.map((part) => (
          <li key={part.key} className="flex items-center gap-1.5 text-[12.5px] leading-[17px]">
            <Swatch colorIndex={NATURES[part.key].colorIndex} />
            <span className="text-ink-soft">{NATURES[part.key].label}</span>
            <span className="font-semibold text-ink">{formatNatureShare(part.euros, total)}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}

/**
 * Programme replié : numéro, nom, montant, barre et part de la mission.
 * Déplié : nature de sa dépense, puis ses actions triées par montant, avec
 * leur part du programme. Un programme à 0 € n'a pas d'action à montrer.
 */
function ProgrammeItem({
  programme,
  missionTotal,
  largest,
  colorIndex,
  initiallyOpen,
}: {
  programme: BudgetProgrammeDto
  missionTotal: number
  largest: number
  colorIndex: number
  initiallyOpen: boolean
}) {
  const expandable = programme.actions.length > 0 && programme.cp_euros > 0
  const [open, setOpen] = useState(initiallyOpen && expandable)
  const largestAction = Math.max(0, ...programme.actions.map((action) => action.cp_euros))
  const color = sliceColor(colorIndex)

  const summary = (
    <>
      <span className="flex w-full items-baseline gap-2.5">
        <span className="min-w-9 shrink-0 rounded-md bg-surface-soft px-1.5 text-center text-xs leading-5 font-semibold text-ink-soft">
          {programme.number}
        </span>
        <span className="flex flex-1 items-center gap-2 text-sm leading-5 text-ink">
          <Swatch colorIndex={colorIndex} />
          {programme.name}
        </span>
        <span className="text-sm leading-5 font-semibold whitespace-nowrap text-ink">
          {formatAmount(programme.cp_euros)}
        </span>
        {expandable ? (
          <Icon
            name="chevron"
            className={`size-4 shrink-0 self-center text-ink-faint transition-transform ${open ? '-rotate-90' : 'rotate-90'}`}
          />
        ) : (
          <span aria-hidden="true" className="size-4 shrink-0" />
        )}
      </span>
      <span className="flex w-full items-center gap-2.5 pl-[46px] pr-[26px]">
        <span className="flex-1">
          <AmountBar width={barWidth(programme.cp_euros, largest)} color={color} />
        </span>
        <span className="w-12 text-right text-[12.5px] leading-4 text-ink-faint">
          {formatProgrammeShare(programme.cp_euros, missionTotal)}
        </span>
      </span>
    </>
  )
  const rowClass = 'flex w-full flex-col gap-1.5 py-2.5 text-left'

  return (
    <li className="border-t border-line">
      {expandable ? (
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
          className={`${rowClass} hover:bg-surface-soft`}
        >
          {summary}
        </button>
      ) : (
        <div className={rowClass}>{summary}</div>
      )}
      {open && (
        <div className="flex flex-col gap-2.5 pb-3">
          <div className="flex flex-col gap-2 rounded-xl bg-surface-soft px-3 py-2.5">
            <p className="text-xs leading-4 text-ink-faint">Nature de la dépense du programme</p>
            <NatureBar natures={programme.nature} total={programme.cp_euros} height="h-2" />
          </div>
          <p className="text-xs leading-4 text-ink-faint">
            {programme.actions.length > 1
              ? `${programme.actions.length} actions, triées par montant`
              : '1 action'}
          </p>
          <ul>
            {programme.actions.map((action) => (
              <li key={action.code} className="flex flex-col gap-1 py-1.5">
                <span className="flex justify-between gap-2.5 text-[13px] leading-[18px]">
                  <span className="text-ink-soft">{action.name}</span>
                  <span className="font-semibold whitespace-nowrap text-ink">
                    {formatAmount(action.cp_euros)}
                  </span>
                </span>
                <span className="flex items-center gap-2">
                  <span className="flex-1">
                    <AmountBar width={barWidth(action.cp_euros, largestAction)} color={color} />
                  </span>
                  <span className="w-11 text-right text-xs text-ink-faint">
                    {formatProgrammeShare(action.cp_euros, programme.cp_euros)}
                  </span>
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </li>
  )
}
