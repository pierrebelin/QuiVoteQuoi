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
import { FamilyIllustration } from '../FamilyIllustration'
import { sliceColor, toSlices } from '../../lib/budgetSlices'
import { NATURES, natureParts } from '../../lib/budgetNature'
import { Donut, Swatch } from './BudgetDonut'

/*
 * Pièces de la page budget : la barre, la ligne d'une mission et son détail
 * (nature de ses crédits, programmes dépliables jusqu'aux actions, sujet). Le
 * détail est le même dans le tiroir du téléphone et dans le panneau de droite
 * à 1280 px.
 */

/** Barre de montant ; `color` reprend la couleur d'une part d'anneau, un gris neutre sinon. */
export function AmountBar({ width, color }: { width: string; color?: string }) {
  return (
    <span aria-hidden="true" className="block h-1.5 overflow-hidden rounded-full bg-surface-soft">
      <span
        className="block h-1.5 rounded-full bg-ink-faint"
        style={color ? { width, backgroundColor: color } : { width }}
      />
    </span>
  )
}

/**
 * Ligne d'une mission, à plat entre deux filets. Sous 1280 px : nom et montant,
 * puis barre et part, sur une grille dont la colonne de droite est de largeur
 * fixe (la barre garde ainsi la même échelle d'une ligne à l'autre). À 1280 px,
 * une ligne de tableau (mission, barre, montant, part) : `order` replace les
 * quatre cellules ; la ligne choisie se détache en cyan pâle.
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
    <li
      data-selected={selected || undefined}
      className="peer border-t border-line data-selected:border-transparent peer-data-selected:border-transparent"
    >
      <button
        type="button"
        onClick={onSelect}
        aria-haspopup={wide ? undefined : 'dialog'}
        aria-pressed={wide ? selected : undefined}
        className={`grid w-full grid-cols-[minmax(0,1fr)_78px] items-start gap-x-3.5 gap-y-2 py-3 text-left transition-colors xl:grid-cols-[minmax(0,300px)_minmax(0,1fr)_88px_56px] xl:items-center xl:gap-4 xl:rounded-xl xl:px-3 xl:py-2 ${
          selected
            ? 'bg-accent-soft ring-[1.5px] ring-accent/40 ring-inset'
            : 'xl:hover:bg-surface'
        }`}
      >
        <span className="text-[15px] leading-5 font-semibold text-ink xl:order-1">
          {mission.name}
        </span>
        <span className="text-right text-[15px] leading-5 font-semibold whitespace-nowrap text-ink xl:order-3">
          {formatAmount(mission.cp_euros)}
        </span>
        <span className="self-center xl:order-2">
          <AmountBar width={barWidth(mission.cp_euros, largest)} />
        </span>
        <span className="self-center text-right text-[13px] leading-4 text-ink-faint xl:order-4">
          {formatMissionShare(mission.cp_euros, total)}
        </span>
      </button>
    </li>
  )
}

/**
 * Ligne d'un compte hors budget général : pas de part, il n'entre pas dans le
 * total. Posée sur une bande grise sous 1280 px, sur le blanc de la page au-delà.
 */
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
    <div
      data-selected={selected || undefined}
      className="border-t border-line data-selected:border-transparent"
    >
      <button
        type="button"
        onClick={onSelect}
        aria-haspopup={wide ? undefined : 'dialog'}
        aria-pressed={wide ? selected : undefined}
        className={`flex w-full flex-col gap-1.5 py-3 text-left transition-colors xl:rounded-xl xl:px-3 ${
          selected ? 'bg-accent-soft ring-[1.5px] ring-accent/40 ring-inset' : 'hover:bg-surface-soft xl:hover:bg-surface'
        }`}
      >
        <span className="flex w-full items-baseline justify-between gap-3">
          <span className="text-[15px] leading-5 font-semibold text-ink">{mission.name}</span>
          <span className="text-[15px] leading-5 font-semibold whitespace-nowrap text-ink">
            {formatAmount(mission.cp_euros)}
          </span>
        </span>
        <span className="text-[13px] leading-[17px] text-ink-faint">{accountLabel(mission)}</span>
      </button>
    </div>
  )
}

/**
 * Programmes de la mission et lien vers son sujet. L'anneau nomme les plus
 * gros programmes ; chaque ligne de la liste prend la couleur de sa part.
 * `onCard` : posé sur la carte grise du panneau (1280 px) plutôt que sur le
 * blanc du tiroir, d'où des fonds d'étiquette et un liseré d'anneau inversés.
 */
export function MissionDetail({
  mission,
  year,
  onCard = false,
}: {
  mission: BudgetMissionDto
  year: number
  onCard?: boolean
}) {
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
    <div className="flex flex-col gap-5">
      <div className="flex items-center gap-4">
        <Donut
          slices={slices}
          size={132}
          onCard={onCard}
          label={`Répartition entre les programmes : ${slices
            .map((slice) => `${slice.label} ${formatProgrammeShare(slice.euros, mission.cp_euros)}`)
            .join(', ')}`}
        >
          <p className="figure text-[19px] leading-6 text-ink">
            {center.value}
            <span className="ml-1 text-[0.55em] font-medium">{center.unit}</span>
          </p>
          <p className="text-[11.5px] leading-[15px] text-ink-faint">en {year}</p>
        </Donut>
        <p className="text-sm leading-[21px] text-ink-soft">
          {mission.account_kind !== 'budget_general'
            ? 'Ce compte est hors budget général : il n’entre pas dans le total.'
            : `Répartition entre ses ${mission.programmes.length} programmes, puis entre leurs ${actionCount} actions.`}
        </p>
      </div>
      <section aria-labelledby="mission-nature" className="flex flex-col gap-2">
        <h3 id="mission-nature" className="text-[15px] leading-5 font-semibold text-ink">
          Ce que paient ces crédits
        </h3>
        <NatureBar natures={mission.nature} total={mission.cp_euros} height="h-3" />
      </section>
      <section aria-labelledby="mission-programmes">
        <h3 id="mission-programmes" className="text-[15px] leading-5 font-semibold text-ink">
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
              onCard={onCard}
            />
          ))}
        </ul>
      </section>
      {mission.theme && (
        <Link
          to={`/themes/${mission.theme.code}`}
          className="flex min-h-14 items-center justify-between gap-3 border-t border-line pt-3"
        >
          <span className="flex flex-col items-start gap-1.5">
            <span className="text-[13px] leading-[17px] text-ink-faint">Sujet QuiVoteQuoi</span>
            <span
              className={`inline-flex h-[26px] items-center gap-1.5 rounded-full pr-2.5 pl-2 text-[13px] font-medium text-ink-soft ${
                onCard ? 'bg-canvas' : 'bg-surface'
              }`}
            >
              <FamilyIllustration
                code={mission.theme.code}
                bare
                className="shrink-0"
                glyphClassName="size-[15px]"
              />
              {mission.theme.label}
            </span>
          </span>
          <Icon name="chevron" className="size-4 text-ink-faint" />
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
  onCard,
}: {
  programme: BudgetProgrammeDto
  missionTotal: number
  largest: number
  colorIndex: number
  initiallyOpen: boolean
  onCard: boolean
}) {
  const expandable = programme.actions.length > 0 && programme.cp_euros > 0
  const [open, setOpen] = useState(initiallyOpen && expandable)
  const largestAction = Math.max(0, ...programme.actions.map((action) => action.cp_euros))
  const color = sliceColor(colorIndex)

  const summary = (
    <>
      <span className="flex w-full items-baseline gap-2.5">
        <span
          className={`min-w-9 shrink-0 rounded-md px-1.5 text-center text-xs leading-5 font-semibold text-ink-soft ${
            onCard ? 'bg-canvas' : 'bg-surface'
          }`}
        >
          {programme.number}
        </span>
        <span className="flex flex-1 items-center gap-2 text-sm leading-5 font-semibold text-ink">
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
          className={`${rowClass} ${onCard ? 'hover:bg-surface-soft' : 'hover:bg-surface'}`}
        >
          {summary}
        </button>
      ) : (
        <div className={rowClass}>{summary}</div>
      )}
      {open && (
        <div className="flex flex-col gap-3 pb-3 pl-[46px]">
          <div className="flex flex-col gap-2 border-l-2 border-line pl-3">
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
