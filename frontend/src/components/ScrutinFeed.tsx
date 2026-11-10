import { useState } from 'react'
import { Link } from 'react-router'
import DossierCardShell, { FamilyChips, OutcomeSplit } from './DossierCardShell'
import PositionGlyph from './entry/PositionGlyph'
import { VoteBar } from './ui'
import { capitalized } from '../lib/entryFlow'
import type { CitedAmendmentDto, ScrutinSummaryDto } from '../types/scrutins'
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

/** Forme du scrutin, tirée du libellé publié : « scrutin public solennel » → « solennel ». */
function ballotKind(label: string) {
  return label.replace(/^scrutin public /, '')
}

function isAdopted(scrutin: ScrutinSummaryDto) {
  return scrutin.outcome_code === 'adopté'
}

/** Ce qui sert de titre : l'amendement mis aux voix, le libellé court, ou le dossier. */
function scrutinTitle(scrutin: ScrutinSummaryDto, inDossier: boolean) {
  const cited = scrutin.cited[0]
  if (cited) return `Amendement n° ${cited.number} · ${cited.target_title}`
  if (scrutin.short_label !== scrutin.subject) return capitalized(scrutin.short_label)
  if (!inDossier && scrutin.dossier_label) return scrutin.dossier_label
  return capitalized(scrutin.subject)
}

/** L'objet officiel reste lisible sous le titre, sauf s'il en est la copie. */
function officialSubject(scrutin: ScrutinSummaryDto, title: string) {
  return title === capitalized(scrutin.subject) ? null : scrutin.subject
}

function familiesOf(scrutins: ScrutinSummaryDto[]): AssignedFamilyDto[] {
  const byCode = new Map<string, AssignedFamilyDto>()
  for (const scrutin of scrutins) for (const family of scrutin.families ?? []) byCode.set(family.code, family)
  return [...byCode.values()]
}

/**
 * Signataire de l'amendement, avec son groupe au dépôt aux couleurs que ce
 * groupe publie (RM-10). Sans couleur publiée, le sigle reste en texte.
 */
function CitedAuthor({ cited, compact = false }: { cited: CitedAmendmentDto; compact?: boolean }) {
  const abbrev = cited.author_group_abbrev
  return (
    <span className="inline-flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5 text-[13px] text-ink-soft">
      {abbrev &&
        (cited.author_group_color ? (
          <span
            className="rounded-md px-1.5 text-[11px] leading-[18px] font-bold tracking-wide ring-1 ring-inset ring-badge-edge"
            style={{
              backgroundColor: cited.author_group_color,
              color: cited.author_text_tone === 'white' ? '#ffffff' : '#000000',
            }}
          >
            {abbrev}
          </span>
        ) : (
          <span className="rounded-md bg-surface-soft px-1.5 text-[11px] leading-[18px] font-semibold ring-1 ring-line ring-inset">
            {abbrev}
          </span>
        ))}
      <span className="text-ink">{cited.author_name ?? 'Auteur non identifié'}</span>
      {cited.cosignatory_count > 0 && (
        <span className="text-ink-faint">
          {compact
            ? `+${cited.cosignatory_count}`
            : `et ${cited.cosignatory_count} cosignataire${cited.cosignatory_count > 1 ? 's' : ''}`}
        </span>
      )}
    </span>
  )
}

function OutcomeTag({ scrutin }: { scrutin: ScrutinSummaryDto }) {
  const adopted = isAdopted(scrutin)
  return (
    <span
      title={scrutin.outcome_label}
      className={`inline-flex w-fit items-center gap-1 rounded-md px-1.5 py-0.5 text-xs font-semibold ${adopted ? 'bg-yes-soft text-yes' : 'bg-no-soft text-no'}`}
    >
      <PositionGlyph position={adopted ? 'for' : 'against'} className="size-3" strokeWidth={3.4} />
      {adopted ? 'Adopté' : 'Rejeté'}
    </span>
  )
}

/** Pour, contre, abstentions en chiffres colorés, puis la barre. */
function Tally({ scrutin, size }: { scrutin: ScrutinSummaryDto; size: 'large' | 'small' }) {
  const { tally } = scrutin
  const notVoting = tally.not_voting
  const figures = [
    ['pour', tally.votes_for, 'text-yes'],
    ['contre', tally.votes_against, 'text-no'],
    ['abst.', tally.abstentions, 'text-abstain'],
  ] as const

  if (size === 'small') {
    return (
      <div className="flex items-center gap-2.5">
        <div className="min-w-0 flex-1">
          <VoteBar tally={tally} />
        </div>
        <span className="flex gap-2 text-[13px] font-semibold tabular-nums">
          {figures.map(([label, value, color]) => (
            <span key={label} className={color} title={label}>
              {value}
            </span>
          ))}
        </span>
      </div>
    )
  }

  return (
    <div>
      <div className="flex gap-5">
        {figures.map(([label, value, color]) => (
          <div key={label}>
            <p className={`text-2xl leading-none font-semibold tracking-tight tabular-nums ${color}`}>{value}</p>
            <p className="mt-1 text-[11.5px] text-ink-faint">{label}</p>
          </div>
        ))}
      </div>
      <div className="mt-2.5">
        <VoteBar tally={tally} />
      </div>
      {notVoting > 0 && (
        <p className="mt-1.5 text-[11.5px] text-ink-faint">
          {notVoting} non-votant{notVoting > 1 ? 's' : ''}
        </p>
      )}
    </div>
  )
}

/**
 * Carte d'un scrutin sans dossier : sans état de dossier à montrer, le
 * bandeau dit l'issue du scrutin. Puis thèmes, sujet, exposé, chiffres.
 */
function ScrutinCard({ scrutin }: { scrutin: ScrutinSummaryDto }) {
  const adopted = isAdopted(scrutin)
  const cited = scrutin.cited[0]
  const title = scrutinTitle(scrutin, false)
  const subject = officialSubject(scrutin, title)
  const kind = ballotKind(scrutin.ballot_type)
  const families = scrutin.families ?? []

  return (
    <article className="group relative overflow-hidden rounded-2xl border border-line bg-surface shadow-card transition-colors hover:border-line-strong">
      <div
        className={`flex flex-wrap items-center justify-between gap-x-4 gap-y-0.5 px-[18px] py-1.5 text-[13px] font-semibold ${adopted ? 'bg-yes-soft text-yes' : 'bg-no-soft text-no'}`}
      >
        <span className="inline-flex items-center gap-1.5" title={scrutin.outcome_label}>
          <PositionGlyph position={adopted ? 'for' : 'against'} className="size-3" strokeWidth={3.2} />
          {adopted ? 'Adopté' : 'Rejeté'}
        </span>
        <span className="text-[12.5px] font-medium text-ink-soft">
          Scrutin n° {scrutin.number} ·{' '}
          {kind === 'ordinaire' ? (
            kind
          ) : (
            <span className="rounded bg-surface px-1.5 text-[11px] font-bold text-ink">{kind}</span>
          )}
        </span>
      </div>

      <div className="grid items-center gap-x-6 gap-y-3 px-[18px] pt-3 pb-3.5 sm:grid-cols-[minmax(0,1fr)_220px]">
        <div className="min-w-0">
          {families.length > 0 && (
            <div className="mb-2">
              <FamilyChips families={families} />
            </div>
          )}

          <h3 className="text-[16.5px] leading-snug font-semibold text-ink transition-colors group-hover:text-accent">
            <Link to={`/scrutins/${scrutin.uid}`} className="after:absolute after:inset-0 focus:outline-none">
              {title}
            </Link>
          </h3>

          {cited && (
            <div className="mt-1">
              <CitedAuthor cited={cited} />
            </div>
          )}
          {cited?.summary_preview && (
            <p className="mt-2 line-clamp-2 border-l-[3px] border-line-strong pl-3 text-[13.5px] leading-relaxed text-ink-soft">
              {cited.summary_preview}
            </p>
          )}
          {scrutin.cited.length > 1 && (
            <p className="mt-1 text-xs text-ink-faint">
              et {scrutin.cited.length - 1} autre{scrutin.cited.length > 2 ? 's' : ''} amendement
              {scrutin.cited.length > 2 ? 's' : ''} mis aux voix ensemble
            </p>
          )}
          {subject && (
            <p className="mt-1.5 text-[12.5px] leading-snug text-ink-faint">Mis aux voix : {subject}</p>
          )}
          {scrutin.has_reconstructed_tallies && (
            <p
              className="mt-1 text-xs text-info"
              title="La source ne publie pas les groupes sur ce scrutin : la répartition est reconstituée."
            >
              Répartition reconstituée
            </p>
          )}
        </div>

        <Tally scrutin={scrutin} size="large" />
      </div>
    </article>
  )
}

/**
 * Une ligne de la carte d'un dossier : issue, sujet, auteur, exposé, objet
 * officiel, chiffres.
 */
function DossierRow({ scrutin }: { scrutin: ScrutinSummaryDto }) {
  const cited = scrutin.cited[0]
  const title = scrutinTitle(scrutin, true)
  const subject = officialSubject(scrutin, title)
  const kind = ballotKind(scrutin.ballot_type)
  return (
    <li className="group relative grid items-center gap-x-4 gap-y-2 border-t border-line px-[18px] py-2.5 transition-colors hover:bg-surface-soft/60 sm:grid-cols-[76px_minmax(0,1fr)_210px]">
      <OutcomeTag scrutin={scrutin} />
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
          <Link
            to={`/scrutins/${scrutin.uid}`}
            className="text-[14.5px] font-semibold text-ink after:absolute after:inset-0 group-hover:text-accent focus:outline-none"
          >
            {title}
          </Link>
          {kind !== 'ordinaire' && (
            <span className="rounded bg-surface-soft px-1.5 text-[11px] leading-[18px] font-bold text-ink">{kind}</span>
          )}
          {cited && <CitedAuthor cited={cited} compact />}
          {scrutin.cited.length > 1 && (
            <span className="text-xs text-ink-faint">
              +{scrutin.cited.length - 1} amendement{scrutin.cited.length > 2 ? 's' : ''}
            </span>
          )}
        </div>
        {cited?.summary_preview && (
          <p className="mt-0.5 truncate text-[12.5px] text-ink-soft">{cited.summary_preview}</p>
        )}
        {subject && (
          <p className="mt-0.5 truncate text-xs text-ink-faint" title={subject}>
            Mis aux voix : {subject}
          </p>
        )}
      </div>
      <Tally scrutin={scrutin} size="small" />
    </li>
  )
}

/**
 * Scrutins successifs d'un même dossier, dans une seule carte. Un scrutin
 * seul n'a pas de répartition : sa ligne dit déjà l'issue.
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
            scope={`${scrutins.length} scrutins ce jour`}
          />
        )
      }
    >
      <ul>
        {shown.map((scrutin) => (
          <DossierRow key={scrutin.uid} scrutin={scrutin} />
        ))}
      </ul>
      {(hidden > 0 || (expanded && scrutins.length > VISIBLE_ROWS)) && (
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
          className="w-full border-t border-line bg-surface-soft/50 px-[18px] py-2 text-left text-[13px] font-medium text-accent hover:bg-surface-soft"
        >
          {expanded
            ? 'Replier'
            : `Afficher les ${hidden} autre${hidden > 1 ? 's' : ''} scrutin${hidden > 1 ? 's' : ''} de ce dossier`}
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
 * Fil des scrutins, un intercalaire par jour : une carte par scrutin, et les
 * scrutins successifs d'un même dossier réunis dans une carte commune sous
 * l'état du dossier.
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
                <ScrutinCard key={entry.scrutin.uid} scrutin={entry.scrutin} />
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
