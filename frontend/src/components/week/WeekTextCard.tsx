import { useState } from 'react'
import DossierCardShell, { OutcomeSplit } from '../DossierCardShell'
import VoteCard from '../VoteCard'
import { scrutinCard } from '../voteCardData'
import type { WeekText } from '../../types/week'

/** Votes visibles avant dépliage, comme dans le fil des scrutins. */
const VISIBLE_ROWS = 3

/**
 * Carte d'un texte de la semaine : étape, sujets, compteurs, puis tous ses
 * votes, dépliables. Avec un jour choisi, les compteurs et les votes sont ceux
 * du jour. Le texte sans dossier a sa carte, sans lien ni étape.
 */
export default function WeekTextCard({
  text,
  day,
  today,
}: {
  text: WeekText
  day: string | null
  today: string
}) {
  const [expanded, setExpanded] = useState(false)
  const scrutins = day ? text.scrutins.filter((scrutin) => scrutin.date === day) : text.scrutins
  const tally = day ? text.days.find((d) => d.date === day)?.tally : text.totals
  if (scrutins.length === 0 || !tally) return null

  const hidden = expanded ? 0 : Math.max(0, scrutins.length - VISIBLE_ROWS)
  const shown = hidden > 0 ? scrutins.slice(0, VISIBLE_ROWS) : scrutins
  const split = (
    <OutcomeSplit
      adopted={tally.adopted}
      rejected={tally.rejected}
      scope={`${tally.votes} vote${tally.votes > 1 ? 's' : ''} ${day ? 'ce jour' : 'cette semaine'}`}
    />
  )
  const rows = (
    <>
      <ul>
        {shown.map((scrutin) => (
          <li key={scrutin.uid} className="border-t border-line">
            <VoteCard size="compact" bare {...scrutinCard(scrutin)} />
          </li>
        ))}
      </ul>
      {(hidden > 0 || (expanded && scrutins.length > VISIBLE_ROWS)) && (
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
          className="w-full border-t border-line bg-surface-soft/50 px-[18px] py-2 text-left text-[13px] font-medium link hover:bg-surface-soft"
        >
          {expanded ? 'Replier' : `Voir les ${hidden} autre${hidden > 1 ? 's' : ''} vote${hidden > 1 ? 's' : ''} de ce texte`}
        </button>
      )}
    </>
  )

  if (!text.dossier_uid) {
    return (
      <article className="overflow-hidden rounded border border-line bg-surface shadow-card">
        <header className="px-[18px] pt-3 pb-3.5">
          <h2 className="text-[16.5px] leading-snug font-semibold text-ink">Votes sans dossier rattaché</h2>
          {split}
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
      split={split}
    >
      {rows}
    </DossierCardShell>
  )
}
