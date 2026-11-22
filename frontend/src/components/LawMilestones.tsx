import type { ReactNode } from 'react'
import type { DecisionSense, LawStateKind, Milestone } from '../types/dossiers'
import { LawStateIcon } from './LawStateBadge'

/** Sens d'une décision de chambre, tel que le sous-titre l'écrit (RM-05). */
const SENSE_WORDS: Record<DecisionSense, string> = {
  adopted: 'adopté',
  adopted_unchanged: 'conforme',
  modified: 'modifié',
  rejected: 'rejeté',
  cmp_agreement: 'accord en CMP',
  cmp_disagreement: 'désaccord en CMP',
}

function stepLabel(milestone: Milestone): string {
  if (milestone.step === 'agreement') return 'Accord'
  if (milestone.step === 'law') return 'Loi'
  return milestone.chamber === 'sn' ? 'Sénat' : 'Assemblée'
}

/**
 * « 7 juil. 2026 ». Sur mobile, l'année ne s'écrit que si elle diffère de
 * celle de la date la plus récente des jalons : quatre colonnes de 74 px ne
 * portent pas « 2026 » quatre fois, mais un « 5 mars » de l'année précédente
 * ne doit pas se lire comme récent.
 */
function MilestoneDate({ iso, referenceYear }: { iso: string; referenceYear: string }) {
  const dayMonth = new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
  })
  const year = iso.slice(0, 4)
  return (
    <span className="whitespace-nowrap">
      {dayMonth}
      {year === referenceYear ? <span className="hidden sm:inline"> {year}</span> : ` ${year}`}
    </span>
  )
}

/** Sous-titre d'un jalon (RM-05, Q7) : sens et date, ou ce qui est attendu. */
function subtitle(milestone: Milestone, date: ReactNode): ReactNode {
  if (milestone.status === 'upcoming') return 'à venir'
  if (milestone.step === 'law') {
    return milestone.status === 'current' ? 'promulgation attendue' : date
  }
  if (milestone.step === 'agreement') {
    const kind =
      milestone.agreement === 'same_text'
        ? 'même texte'
        : milestone.agreement === 'final_reading'
          ? 'lecture définitive'
          : milestone.agreement === 'joint_committee'
            ? milestone.sense === 'cmp_disagreement'
              ? 'désaccord en CMP'
              : 'CMP'
            : null
    if (kind && date) return <>{kind} · {date}</>
    return kind ?? date ?? 'à venir'
  }
  const sense = milestone.sense ? SENSE_WORDS[milestone.sense] : null
  if (sense && date) return <>{sense} {date}</>
  return sense ?? date ?? 'à venir'
}

/**
 * Marque d'un jalon : point noir pour une étape franchie, icône d'état sur
 * l'étape en cours ou sur la loi publiée, cercle en pointillé pour une étape à
 * venir (RM-05).
 */
function Marker({ milestone, stateKind }: { milestone: Milestone; stateKind: LawStateKind }) {
  const base =
    'absolute top-0 left-0 flex size-5 items-center justify-center rounded-full box-border'
  if (milestone.step === 'law' && stateKind === 'became_law') {
    return (
      <span className={`${base} bg-yes text-surface`}>
        <LawStateIcon kind="became_law" className="size-2.5" bold />
      </span>
    )
  }
  if (milestone.status === 'current') {
    const ring = stateKind === 'in_shuttle' ? 'border-ink text-ink' : 'border-yes text-yes'
    return (
      <span className={`${base} border-2 bg-surface ${ring}`}>
        <LawStateIcon kind={stateKind} className="size-2.5" bold />
      </span>
    )
  }
  if (milestone.status === 'done') {
    return (
      <span className={`${base} bg-surface`}>
        <span className="size-3 rounded-full bg-ink" />
      </span>
    )
  }
  return <span className={`${base} border-2 border-dashed border-line-strong bg-surface`} />
}

/**
 * Les quatre jalons d'un dossier : chambre de dépôt, autre chambre, accord,
 * loi. Sur la fiche de vote, l'étape tranchée par le scrutin porte « · ce
 * vote ».
 */
export default function LawMilestones({
  milestones,
  stateKind,
  className = '',
}: {
  milestones: Milestone[]
  stateKind: LawStateKind
  className?: string
}) {
  const referenceYear =
    milestones
      .map((m) => m.date)
      .filter((date): date is string => date !== null)
      .sort()
      .at(-1)
      ?.slice(0, 4) ?? ''

  return (
    <ol aria-label="Avancement du texte" className={`grid max-w-[620px] grid-cols-4 ${className}`}>
      {milestones.map((milestone, index) => {
        const last = index === milestones.length - 1
        const date = milestone.date ? (
          <MilestoneDate iso={milestone.date} referenceYear={referenceYear} />
        ) : null
        return (
          <li
            key={`${milestone.step}-${milestone.chamber ?? index}`}
            className="relative pt-7 pr-1.5 text-xs leading-4 sm:text-[13px] sm:leading-[18px]"
          >
            {!last && (
              <span
                aria-hidden
                className={`absolute top-[9px] right-0 left-0 h-0.5 ${milestone.status === 'done' ? 'bg-ink' : 'bg-line'}`}
              />
            )}
            <Marker milestone={milestone} stateKind={stateKind} />
            <span className="block font-semibold text-ink">{stepLabel(milestone)}</span>
            <span className="block text-ink-soft">
              {subtitle(milestone, date)}
              {milestone.is_this_vote && (
                <>
                  {' '}
                  <b className="font-semibold whitespace-nowrap text-ink">· ce vote</b>
                </>
              )}
            </span>
          </li>
        )
      })}
    </ol>
  )
}
