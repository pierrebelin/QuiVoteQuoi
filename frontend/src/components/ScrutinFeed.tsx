import { useState } from 'react'
import DossierCardShell, { OutcomeSplit } from './DossierCardShell'
import VoteCard from './VoteCard'
import { scrutinCard } from './voteCardData'
import { capitalized } from '../lib/entryFlow'
import type { ScrutinSummaryDto } from '../types/scrutins'
import type { AssignedFamilyDto } from '../types/themes'

/**
 * Lignes visibles d'un dossier avant dépliage : une séance budgétaire en
 * enchaîne des dizaines, et sur mobile trois lignes tiennent déjà un écran.
 */
const VISIBLE_ROWS = 3

type Entry =
  | { kind: 'single'; scrutin: ScrutinSummaryDto }
  | { kind: 'dossier'; dossierUid: string; scrutins: ScrutinSummaryDto[] }

interface Day {
  date: string
  entries: Entry[]
}

/**
 * Jours, puis dans chaque jour les scrutins successifs d'un même dossier
 * réunis. L'ordre de la source n'est jamais changé : seuls des voisins sont
 * regroupés. Un scrutin seul de son dossier a aussi une carte de dossier ;
 * seul un scrutin sans dossier garde sa carte propre.
 */
function groupFeed(scrutins: ScrutinSummaryDto[]): Day[] {
  const days: Day[] = []
  for (const scrutin of scrutins) {
    let day = days[days.length - 1]
    if (!day || day.date !== scrutin.date) {
      day = { date: scrutin.date, entries: [] }
      days.push(day)
    }
    const last = day.entries[day.entries.length - 1]
    if (!scrutin.dossier_uid) {
      day.entries.push({ kind: 'single', scrutin })
    } else if (last?.kind === 'dossier' && last.dossierUid === scrutin.dossier_uid) {
      last.scrutins.push(scrutin)
    } else {
      day.entries.push({ kind: 'dossier', dossierUid: scrutin.dossier_uid, scrutins: [scrutin] })
    }
  }
  return days
}

function dayLabel(iso: string) {
  return capitalized(
    new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
      weekday: 'long',
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    }),
  )
}

function isAdopted(scrutin: ScrutinSummaryDto) {
  return scrutin.outcome_code === 'adopté'
}

function familiesOf(scrutins: ScrutinSummaryDto[]): AssignedFamilyDto[] {
  const byCode = new Map<string, AssignedFamilyDto>()
  for (const scrutin of scrutins) for (const family of scrutin.families ?? []) byCode.set(family.code, family)
  return [...byCode.values()]
}

/**
 * Votes successifs d'un même texte, en cartes compactes sous l'en-tête du
 * texte. Un vote seul n'a pas de répartition : sa carte dit déjà l'issue.
 */
function DossierRun({
  dossierUid,
  scrutins,
  today,
}: {
  dossierUid: string
  scrutins: ScrutinSummaryDto[]
  today: string
}) {
  const [expanded, setExpanded] = useState(false)
  const adopted = scrutins.filter(isAdopted).length
  const hidden = expanded ? 0 : Math.max(0, scrutins.length - VISIBLE_ROWS)
  const shown = hidden > 0 ? scrutins.slice(0, VISIBLE_ROWS) : scrutins

  return (
    <DossierCardShell
      uid={dossierUid}
      title={scrutins[0].dossier_label ?? dossierUid}
      status={scrutins[0].dossier_status}
      today={today}
      families={familiesOf(scrutins)}
      split={
        scrutins.length > 1 && (
          <OutcomeSplit
            adopted={adopted}
            rejected={scrutins.length - adopted}
            scope={`${scrutins.length} votes ce jour`}
          />
        )
      }
    >
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
          {expanded
            ? 'Replier'
            : `Voir les ${hidden} autre${hidden > 1 ? 's' : ''} vote${hidden > 1 ? 's' : ''} de ce texte`}
        </button>
      )}
    </DossierCardShell>
  )
}

function DayHeading({ date }: { date: string }) {
  return (
    <h2 className="mb-2.5 flex items-center gap-3 text-[15px] font-semibold text-ink">
      <time dateTime={date}>{dayLabel(date)}</time>
      <span aria-hidden className="h-px flex-1 bg-line" />
    </h2>
  )
}

/**
 * Fil des votes, un intercalaire par jour : une carte compacte par vote, et
 * les votes successifs d'un même texte réunis sous l'en-tête de ce texte.
 */
export default function ScrutinFeed({ scrutins }: { scrutins: ScrutinSummaryDto[] }) {
  const today = new Date().toISOString().slice(0, 10)

  return (
    <div>
      {groupFeed(scrutins).map((day) => (
        <section key={day.date} className="mt-6 first:mt-0">
          <DayHeading date={day.date} />
          <div className="space-y-3">
            {day.entries.map((entry) =>
              entry.kind === 'single' ? (
                <VoteCard key={entry.scrutin.uid} size="compact" {...scrutinCard(entry.scrutin)} />
              ) : (
                <DossierRun
                  key={entry.scrutins[0].uid}
                  dossierUid={entry.dossierUid}
                  scrutins={entry.scrutins}
                  today={today}
                />
              ),
            )}
          </div>
        </section>
      ))}
    </div>
  )
}
