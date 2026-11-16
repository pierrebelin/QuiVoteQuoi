import { Link } from 'react-router'
import type { BudgetMissionDto } from '../../types/budget'
import {
  barWidth,
  formatAmount,
  formatMissionShare,
  formatProgrammeShare,
  accountLabel,
} from '../../lib/budgetFormat'
import { Icon } from '../icons'

/*
 * Pièces de la page budget : la barre, la ligne d'une mission et son détail
 * (programmes et sujet). Le détail est le même dans le tiroir du téléphone et
 * dans le panneau de droite à 1280 px.
 */

export function AmountBar({ width }: { width: string }) {
  return (
    <span aria-hidden="true" className="block h-2 overflow-hidden rounded-full bg-surface-soft">
      <span className="block h-2 rounded-full bg-accent" style={{ width }} />
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

/** Programmes de la mission et lien vers son sujet. */
export function MissionDetail({ mission }: { mission: BudgetMissionDto }) {
  const largest = Math.max(0, ...mission.programmes.map((programme) => programme.cp_euros))
  return (
    <div className="flex flex-col gap-3.5">
      {mission.account_kind !== 'budget_general' && (
        <p className="text-[13px] leading-[19px] text-ink-soft">
          Ce compte est hors budget général : il n’entre pas dans le total.
        </p>
      )}
      <ul className="flex flex-col border-b border-line">
        {mission.programmes.map((programme) => (
          <li key={programme.number} className="flex flex-col gap-1.5 border-t border-line py-2.5">
            <span className="flex items-baseline gap-2.5">
              <span className="min-w-9 shrink-0 rounded-md bg-surface-soft px-1.5 text-center text-xs leading-5 font-semibold text-ink-soft">
                {programme.number}
              </span>
              <span className="flex-1 text-sm leading-5 text-ink">{programme.name}</span>
              <span className="text-sm leading-5 font-semibold whitespace-nowrap text-ink">
                {formatAmount(programme.cp_euros)}
              </span>
            </span>
            <span className="flex items-center gap-2.5 pl-[46px]">
              <span className="flex-1">
                <AmountBar width={barWidth(programme.cp_euros, largest)} />
              </span>
              <span className="w-12 text-right text-[12.5px] leading-4 text-ink-faint">
                {formatProgrammeShare(programme.cp_euros, mission.cp_euros)}
              </span>
            </span>
          </li>
        ))}
      </ul>
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
