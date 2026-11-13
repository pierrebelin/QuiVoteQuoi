import { useState } from 'react'
import DossierCardShell, { OutcomeSplit } from '../DossierCardShell'
import PositionGlyph from '../entry/PositionGlyph'
import VoteCard from '../VoteCard'
import { capitalized } from '../../lib/entryFlow'
import { agreed, scrutinCard } from '../voteCardData'
import type { WeekCardTally, WeekText } from '../../types/week'

const votes = (count: number) => `${count} vote${count > 1 ? 's' : ''}`

/** « Lun. 9 : 34 votes · mar. 10 : 13 votes », jours sans vote omis. */
function daySpread(days: WeekText['days']): string {
  return days
    .filter((d) => d.tally.votes > 0)
    .map((d, index) => {
      const label = new Date(d.date + 'T00:00:00')
        .toLocaleDateString('fr-FR', { weekday: 'short', day: 'numeric' })
        .replace(/^(\p{L}+)\.? /u, '$1. ')
      return `${index === 0 ? capitalized(label) : label} : ${votes(d.tally.votes)}`
    })
    .join(' · ')
}

/** « Dont le vote sur l'ensemble, Rejeté » : sort du plus récent de la période (RM-08). */
function LatestFinal({ final, textLabel }: { final: NonNullable<WeekCardTally['latest_final']>; textLabel: string }) {
  const adopted = final.outcome_code === 'adopté'
  return (
    <p className="mt-2 flex flex-wrap items-center gap-1.5 text-[13px] text-ink-soft">
      Dont le vote sur l’ensemble,
      <span
        title={capitalized(final.outcome_label)}
        className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[12.5px] font-semibold ${
          adopted ? 'bg-yes-soft text-yes' : 'bg-no-soft text-no'
        }`}
      >
        <PositionGlyph position={adopted ? 'for' : 'against'} className="size-3" strokeWidth={3.2} />
        {agreed(adopted, textLabel)}
      </span>
    </p>
  )
}

/**
 * Carte d'un texte de la semaine : étape, sujets, compteurs, répartition par
 * jour, sort du vote sur l'ensemble, puis ses votes, repliés à l'ouverture
 * (RM-10). Avec un jour choisi, tout porte sur ce jour (RM-11). Les votes sans
 * dossier ont leur carte, sans lien ni étape (RM-09).
 */
export default function WeekTextCard({
  text,
  day,
  today,
  finalLabels,
}: {
  text: WeekText
  day: string | null
  today: string
  /** Intitulé de chaque vote sur l'ensemble de la semaine, pour accorder son sort. */
  finalLabels: Map<string, string>
}) {
  const [expanded, setExpanded] = useState(false)
  const scrutins = day ? text.scrutins.filter((scrutin) => scrutin.date === day) : text.scrutins
  const tally = day ? text.days.find((d) => d.date === day)?.tally : text.totals
  if (scrutins.length === 0 || !tally) return null

  const days = day ? text.days.filter((d) => d.date === day) : text.days
  const final = tally.latest_final
  const summary = (
    <>
      <OutcomeSplit
        adopted={tally.adopted}
        rejected={tally.rejected}
        scope={`${votes(tally.votes)} ${day ? 'ce jour' : 'cette semaine'}`}
      />
      <p className="mt-1.5 text-[12.5px] text-ink-faint">{daySpread(days)}</p>
      {final && (
        <LatestFinal final={final} textLabel={finalLabels.get(final.scrutin_uid) ?? text.dossier_label ?? ''} />
      )}
    </>
  )
  const rows = (
    <>
      {expanded && (
        <ul>
          {scrutins.map((scrutin) => (
            <li key={scrutin.uid} className="border-t border-line">
              <VoteCard size="compact" bare {...scrutinCard(scrutin)} />
            </li>
          ))}
        </ul>
      )}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
        className="flex min-h-11 w-full items-center justify-center border-t border-line text-[13.5px] font-semibold text-ink transition-colors hover:bg-surface-soft/50"
      >
        {expanded
          ? 'Replier'
          : scrutins.length === 1
            ? 'Voir le vote de ce texte'
            : `Voir les ${scrutins.length} votes de ce texte`}
      </button>
    </>
  )

  if (!text.dossier_uid) {
    return (
      <article className="overflow-hidden rounded border border-line bg-surface shadow-card">
        <header className="px-[18px] pt-3 pb-3.5">
          <h2 className="text-[16.5px] leading-snug font-semibold text-ink">Votes sans texte rattaché</h2>
          <p className="mt-1 text-[12.5px] text-ink-soft">La source ne publie pas de texte rattaché pour ces votes.</p>
          {summary}
        </header>
        {rows}
      </article>
    )
  }

  return (
    <DossierCardShell
      uid={text.dossier_uid}
      title={text.dossier_label ?? text.dossier_uid}
      status={text.dossier_status}
      today={today}
      families={text.families}
      split={summary}
    >
      {rows}
    </DossierCardShell>
  )
}
