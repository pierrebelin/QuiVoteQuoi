import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { GroupDot } from '../pages/GroupListPage'
import { GROUP_LOGOS } from '../content/logos'
import { InfoTip } from './InfoTip'
import { PoliticalLogo } from './PoliticalLogo'
import { Icon } from './icons'
import { Card, ErrorPanel, Pill, TallyLine, VoteBar } from './ui'
import { leadingPosition, positionLabels } from '../types/scrutins'
import type {
  DossierGroupActionsResponse,
  FateCountDto,
  GroupActionsDto,
  SummarySourceDto,
} from '../types/dossierGroupActions'

const MAX_COMPARED_GROUPS = 4
const COMPARE_HINT =
  'Touchez un groupe pour le sort de ses amendements et ses votes de chaque lecture ; cochez « Comparer » pour mettre jusqu\'à quatre groupes côte à côte.'

function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/** Lien vers les amendements du groupe, dans l'onglet dédié de la fiche. */
function amendmentsHref(group: GroupActionsDto) {
  return group.amendments_url.replace('?', '?tab=amendements&')
}

function hasActions(group: GroupActionsDto) {
  return group.final_votes.length > 0 || group.amendment_count > 0
}

function SourceLinks({ sources }: { sources: SummarySourceDto[] }) {
  return (
    <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-[13px]">
      <span className="text-ink-faint">Sources :</span>
      {sources.map((source) =>
        source.official_url ? (
          <a
            key={source.id}
            href={source.official_url}
            target="_blank"
            rel="noreferrer"
            className="link"
          >
            {source.label} ↗
          </a>
        ) : (
          <span key={source.id} className="text-ink-soft" title={source.id}>
            {source.label}
          </span>
        ),
      )}
    </div>
  )
}

/**
 * Sorts d'amendements d'un groupe, avec le libellé de la source, dans l'ordre
 * où l'API les donne. Refonte A : le nombre en chiffres Bricolage, son libellé
 * dessous ; ni vert ni rouge, et jamais la couleur du groupe. Dans le balisage
 * le libellé précède le nombre, comme avant ; l'affichage les range à l'envers.
 */
/** Ordre fixe des sorts, le même pour chaque groupe ; un sort hors liste suit, par nombre décroissant. */
const FATE_ORDER = ['Adopté', 'Rejeté', 'Tombé', 'Retiré', 'Non soutenu', 'Sort non publié']

function fateLabel(fate: FateCountDto) {
  return fate.label || 'Sort non publié'
}

function fateRank(fate: FateCountDto) {
  const rank = FATE_ORDER.indexOf(fateLabel(fate))
  return rank === -1 ? FATE_ORDER.length : rank
}

function FateStats({ fates }: { fates: FateCountDto[] }) {
  const ordered = [...fates].sort((a, b) => fateRank(a) - fateRank(b) || b.count - a.count)
  return (
    <ul className="grid grid-cols-3 gap-x-2.5 gap-y-3 lg:grid-cols-3">
      {ordered.map((fate) => (
        <li key={fate.label} className="flex min-w-0 flex-col-reverse border-l-2 border-line pl-2.5">
          <span className="text-[13px] leading-tight text-ink-soft">{fateLabel(fate)}</span>
          <span className="figure text-xl tabular-nums text-ink">{fate.count.toLocaleString('fr-FR')}</span>
        </li>
      ))}
    </ul>
  )
}

type Position = keyof typeof positionLabels

function isPosition(value: string): value is Position {
  return value in positionLabels
}

function positionLabel(value: string) {
  return isPosition(value) ? positionLabels[value] : value
}

type FinalVoteDto = GroupActionsDto['final_votes'][number]

/** Votes finaux du texte, une entrée par scrutin, du plus ancien au plus récent. */
function distinctFinalVotes(groups: GroupActionsDto[]) {
  const byUid = new Map<string, FinalVoteDto>()
  for (const group of groups) {
    for (const vote of group.final_votes) {
      if (!byUid.has(vote.scrutin_uid)) byUid.set(vote.scrutin_uid, vote)
    }
  }
  return [...byUid.values()].sort((a, b) => a.date.localeCompare(b.date) || Number(a.number) - Number(b.number))
}

function finalVoteLabel(vote: FinalVoteDto) {
  if (vote.reading) return vote.reading.charAt(0).toUpperCase() + vote.reading.slice(1)
  const day = new Date(vote.date + 'T00:00:00').toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
  return `Vote du ${day}`
}

/**
 * Décompte court d'un groupe : les seules positions exprimées, sans adverbe
 * (README.md §6). Les non-votants restent dans le détail du groupe.
 */
function ShortTally({ tally }: { tally: FinalVoteDto['tally'] }) {
  const parts = [
    { key: 'for', value: tally.votes_for, label: 'pour', className: 'text-yes' },
    { key: 'against', value: tally.votes_against, label: 'contre', className: 'text-no' },
    { key: 'abstention', value: tally.abstentions, label: 'abst.', className: 'text-abstain' },
  ].filter((part) => part.value > 0)
  if (parts.length === 0) return <span className="text-[13px] text-ink-faint">aucun votant</span>
  return (
    <span className="text-[13px] leading-snug text-ink-soft tabular-nums">
      {parts.map((part, index) => (
        <span key={part.key}>
          {index > 0 && ' · '}
          {/* Un segment ne se coupe jamais : la ligne passe à la ligne entre deux positions. */}
          <span className="whitespace-nowrap">
            <b className={`font-semibold ${part.className}`}>{part.value}</b> {part.label}
          </span>
        </span>
      ))}
    </span>
  )
}

/** Choix du vote final montré dans la colonne du tableau, quand le texte en a plusieurs. */
function FinalVoteSwitch({
  votes,
  selected,
  onSelect,
}: {
  votes: FinalVoteDto[]
  selected: string
  onSelect: (scrutinUid: string) => void
}) {
  return (
    <div role="group" aria-label="Vote final affiché" className="flex flex-wrap gap-1 rounded-[14px] bg-surface p-1 md:inline-flex">
      {votes.map((vote) => {
        const active = vote.scrutin_uid === selected
        return (
          <button
            key={vote.scrutin_uid}
            type="button"
            aria-pressed={active}
            onClick={() => onSelect(vote.scrutin_uid)}
            title={`Scrutin n° ${vote.number} du ${formatDate(vote.date)}`}
            className={`min-h-10 flex-1 rounded-[10px] px-3 text-[13px] font-semibold transition-colors md:flex-none ${
              active ? 'bg-canvas text-ink shadow-[0_0_0_1px_var(--color-line)]' : 'text-ink-soft hover:text-ink'
            }`}
          >
            {finalVoteLabel(vote)}
          </button>
        )
      })}
    </div>
  )
}

/** Colonnes du tableau des groupes : logo, groupe, amendements, vote final, chevron. */
const GROUP_TABLE_COLS =
  'grid-cols-[40px_64px_40px_minmax(0,1fr)_18px] gap-x-2.5 md:grid-cols-[56px_minmax(0,1fr)_96px_240px_76px] md:gap-x-5'

function FinalVote({ vote }: { vote: GroupActionsDto['final_votes'][number] }) {
  return (
    <div className="rounded-[14px] bg-canvas p-3.5">
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <Link to={`/scrutins/${vote.scrutin_uid}`} className="text-sm font-semibold link">
          Scrutin n° {vote.number}
        </Link>
        <span className="text-[13px] text-ink-faint">{formatDate(vote.date)}</span>
        {vote.reading && <span className="text-[13px] text-ink-soft">· {vote.reading}</span>}
        <Pill tone={vote.outcome_code === 'adopté' ? 'yes' : vote.outcome_code === 'rejeté' ? 'no' : 'neutral'}>
          {vote.outcome_label}
        </Pill>
      </div>
      <p className="mt-1.5 text-[13px] leading-relaxed text-ink-soft">{vote.text_label}</p>
      <div className="mt-2.5">
        <VoteBar tally={vote.tally} />
      </div>
      <div className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1">
        <TallyLine tally={vote.tally} />
        {vote.majority_position && vote.majority_position !== leadingPosition(vote.tally) && (
          <span className="text-[13px] text-ink-soft">
            Position majoritaire publiée par l'Assemblée : {positionLabel(vote.majority_position).toLowerCase()}
          </span>
        )}
        <a href={vote.official_url} target="_blank" rel="noreferrer" className="text-[13px] font-semibold link">
          Source officielle ↗
        </a>
      </div>
      {vote.explanations.length > 0 && (
        <ul className="mt-1.5 space-y-0.5">
          {vote.explanations.map((mention) => (
            <li key={`${mention.scrutin_uid}-${mention.rank}`} className="text-[13px] text-ink-soft">
              Explication de vote publiée —{' '}
              <Link to={`/scrutins/${mention.scrutin_uid}#explication-${mention.rank}`} className="link">
                {mention.speaker_name ?? mention.speaker_uid}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

/** Synthèse et votes finaux d'un groupe, tels que publiés. */
function GroupDetails({ group }: { group: GroupActionsDto }) {
  return (
    <div className="space-y-3">
      {(group.start_date || group.end_date) && (
        <p className="text-[13px] text-ink-faint">
          Groupe concerné du {group.start_date ? formatDate(group.start_date) : 'début de période'} au{' '}
          {group.end_date ? formatDate(group.end_date) : '—'}
        </p>
      )}
      {group.summary ? (
        <div className="rounded-[14px] bg-canvas p-3.5">
          <Pill tone="info">{group.summary.label}</Pill>
          <p className="mt-1.5 text-sm leading-relaxed text-ink-soft">{group.summary.text}</p>
          <SourceLinks sources={group.summary.sources} />
        </div>
      ) : group.state === 'summary_pending' ? (
        <p className="text-[13px] text-ink-faint">
          Synthèse automatique en attente de génération. Les actes restent consultables.
        </p>
      ) : group.state === 'summary_unavailable' ? (
        <p className="text-[13px] text-ink-faint">
          Synthèse automatique indisponible pour le moment. Les actes restent consultables.
        </p>
      ) : null}
      <div>
        <p className="mb-2 text-[13px] font-semibold text-ink-faint">
          Votes finaux, par lecture
        </p>
        {group.final_votes.length === 0 ? (
          <p className="text-[13px] text-ink-faint">Aucun vote final publié.</p>
        ) : (
          <div className="space-y-2">
            {group.final_votes.map((vote) => (
              <FinalVote key={vote.scrutin_uid} vote={vote} />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function CompareToggle({
  checked,
  disabled,
  onChange,
}: {
  checked: boolean
  disabled: boolean
  onChange: () => void
}) {
  return (
    <label
      className={`inline-flex min-h-11 shrink-0 items-center gap-2.5 text-sm font-semibold text-ink ${disabled ? 'cursor-not-allowed opacity-40' : 'cursor-pointer'}`}
    >
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={onChange}
        className="size-5 rounded-md accent-ink"
      />
      Comparer
    </label>
  )
}

/**
 * Ligne d'un groupe qui a agi sur le texte (refonte compacte, variante A) :
 * logo, groupe, nombre d'amendements, décompte du vote final choisi. Le sort
 * des amendements, les votes de chaque lecture et la comparaison sont dans le
 * dépliant. Les groupes restent dans l'ordre de la source, et chaque barre se
 * normalise sur son propre scrutin : aucun classement, aucune échelle commune
 * entre groupes (README.md §6).
 */
function GroupRow({
  group,
  shownVote,
  compared,
  compareDisabled,
  onCompare,
}: {
  group: GroupActionsDto
  shownVote: string | null
  compared: boolean
  compareDisabled: boolean
  onCompare: () => void
}) {
  const fates = group.amendment_fates ?? []
  const vote = group.final_votes.find((candidate) => candidate.scrutin_uid === shownVote)
  const amendments = group.amendment_count
  return (
    <details className="group/row border-b border-line">
      <summary
        title={group.label}
        className={`grid min-h-14 cursor-pointer list-none items-center py-2 ${GROUP_TABLE_COLS} [&::-webkit-details-marker]:hidden`}
      >
        <PoliticalLogo
          logo={GROUP_LOGOS[group.abbrev]}
          fallback={group.abbrev}
          color={group.color}
          className="h-[26px] w-10 md:h-9 md:w-14"
        />
        <span className="min-w-0">
          <span className="block truncate text-sm font-semibold text-ink md:hidden">{group.abbrev}</span>
          <span className="hidden truncate text-base leading-tight font-semibold text-ink md:block">{group.label}</span>
          <span className="hidden text-[13px] text-ink-faint md:block">{group.abbrev}</span>
        </span>
        <span className="text-right">
          {amendments > 0 ? (
            <span className="figure text-[17px] text-ink tabular-nums md:text-xl">
              {amendments.toLocaleString('fr-FR')}
              <span className="sr-only"> amendement{amendments > 1 ? 's' : ''}</span>
            </span>
          ) : (
            <span className="text-[13px] text-ink-faint" aria-label="aucun amendement">
              —
            </span>
          )}
        </span>
        <span className="flex min-w-0 flex-col gap-1">
          {vote ? (
            <>
              <ShortTally tally={vote.tally} />
              <span aria-hidden="true" className="block">
                <VoteBar tally={vote.tally} />
              </span>
            </>
          ) : (
            <span className="text-[13px] text-ink-faint">pas de vote publié</span>
          )}
        </span>
        <span className="inline-flex items-center justify-end gap-1.5 text-sm font-semibold text-ink">
          <span className="hidden md:inline md:group-open/row:hidden">Détails</span>
          <span className="hidden md:group-open/row:inline">Replier</span>
          <Icon name="down" className="size-[18px] text-ink-faint transition-transform group-open/row:rotate-180" strokeWidth={2} />
        </span>
      </summary>
      <div className="mb-4 space-y-4 rounded-[14px] bg-surface p-3.5 md:mb-5 md:p-5">
        {fates.length > 0 && (
          <div>
            <p className="mb-2 text-[13px] font-semibold text-ink-faint">
              Sort de ses {amendments.toLocaleString('fr-FR')} amendement{amendments > 1 ? 's' : ''}
            </p>
            <FateStats fates={fates} />
          </div>
        )}
        <GroupDetails group={group} />
        <div className="flex flex-wrap items-center justify-between gap-x-4">
          <CompareToggle checked={compared} disabled={compareDisabled} onChange={onCompare} />
          {amendments > 0 && (
            <Link to={amendmentsHref(group)} className="inline-flex min-h-11 items-center text-sm font-semibold link">
              Voir ses {amendments.toLocaleString('fr-FR')} amendement{amendments > 1 ? 's' : ''} →
            </Link>
          )}
        </div>
      </div>
    </details>
  )
}

function comparisonGridCols(count: number) {
  if (count >= 4) return 'sm:grid-cols-2 xl:grid-cols-4'
  if (count === 3) return 'sm:grid-cols-3'
  if (count === 2) return 'sm:grid-cols-2'
  return ''
}

/** Fiche alignée d'un groupe comparé : faits publiés, sans score ni conclusion. */
function ComparedGroup({ group, onRemove }: { group: GroupActionsDto; onRemove: () => void }) {
  return (
    <Card className="min-w-0 p-4">
      <div className="mb-3 flex items-center gap-3">
        <PoliticalLogo
          logo={GROUP_LOGOS[group.abbrev]}
          fallback={group.abbrev}
          color={group.color}
          className="h-9 w-14"
        />
        <div className="min-w-0 flex-1">
          <h4 className="text-base leading-tight font-semibold text-ink" title={group.label}>
            {group.label}
          </h4>
          <p className="text-[13px] text-ink-faint">{group.abbrev}</p>
        </div>
        <button
          type="button"
          onClick={onRemove}
          aria-label={`Retirer ${group.abbrev} de la comparaison`}
          className="flex size-11 shrink-0 items-center justify-center rounded text-lg leading-none text-ink-faint hover:bg-surface-soft hover:text-ink"
        >
          ×
        </button>
      </div>
      {hasActions(group) ? (
        <>
          {(group.amendment_fates ?? []).length > 0 && (
            <div className="mb-4">
              <FateStats fates={group.amendment_fates ?? []} />
            </div>
          )}
          <GroupDetails group={group} />
        </>
      ) : (
        <p className="text-[13px] leading-relaxed text-ink-faint">
          Aucun vote final ni amendement associé n'est disponible pour ce groupe. Cette absence ne vaut pas
          abstention.
        </p>
      )}
    </Card>
  )
}

export default function DossierGroupActions({ uid }: { uid: string }) {
  const [params, setParams] = useSearchParams()
  const { data, isLoading, isError, error } = useQuery<DossierGroupActionsResponse>({
    queryKey: ['dossier-lecture-groupes', uid],
    queryFn: () =>
      fetch(`/api/dossiers/${uid}/lecture-groupes`).then((response) => {
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        return response.json()
      }),
    enabled: !!uid,
  })

  const selectedUids = (params.get('avis_groupes') ?? '')
    .split(',')
    .filter(Boolean)
    .slice(0, MAX_COMPARED_GROUPS)
  const selected = data?.groups.filter((group) => selectedUids.includes(group.uid)) ?? []
  const full = selectedUids.length >= MAX_COMPARED_GROUPS
  const [chosenVote, setChosenVote] = useState<string | null>(null)

  function toggleGroup(groupUid: string) {
    const next = selectedUids.includes(groupUid)
      ? selectedUids.filter((uid) => uid !== groupUid)
      : [...selectedUids, groupUid].slice(0, MAX_COMPARED_GROUPS)
    const updated = new URLSearchParams(params)
    if (next.length === 0) updated.delete('avis_groupes')
    else updated.set('avis_groupes', next.join(','))
    setParams(updated, { replace: true, preventScrollReset: true })
  }

  if (isLoading) return <p className="animate-pulse text-sm text-ink-faint">Chargement des actes par groupe…</p>
  if (isError) return <ErrorPanel error={error} />
  if (!data) return null

  const active = data.groups.filter(hasActions)
  const quiet = data.groups.filter((group) => !hasActions(group))
  const finalVotes = distinctFinalVotes(active)
  // Par défaut, le vote final le plus récent.
  const shownVote = finalVotes.find((vote) => vote.scrutin_uid === chosenVote) ?? finalVotes.at(-1) ?? null

  return (
    <div className="space-y-8">
      {selected.length > 0 && (
        <section>
          <div className="mb-3 flex items-baseline justify-between gap-3">
            <h3 className="heading-5">
              Groupes comparés {selected.length}/{MAX_COMPARED_GROUPS}
            </h3>
            <button
              type="button"
              onClick={() => {
                const updated = new URLSearchParams(params)
                updated.delete('avis_groupes')
                setParams(updated, { replace: true, preventScrollReset: true })
              }}
              className="inline-flex min-h-11 items-center text-sm font-semibold link"
            >
              Tout retirer
            </button>
          </div>
          <div className={`grid min-w-0 gap-3 ${comparisonGridCols(selected.length)}`}>
            {selected.map((group) => (
              <ComparedGroup key={group.uid} group={group} onRemove={() => toggleGroup(group.uid)} />
            ))}
          </div>
        </section>
      )}

      <section>
        <h3 className="heading-4 md:heading-3">
          Ce que chaque groupe a fait sur ce texte
          <InfoTip label="Comment lire cette section" className="md:hidden">
            <p>{COMPARE_HINT}</p>
            <p>{data.notes.join(' ')}</p>
          </InfoTip>
        </h3>
        <p className="mt-2 hidden max-w-[640px] text-sm leading-relaxed text-ink-soft md:block">{COMPARE_HINT}</p>

        {active.length > 0 ? (
          <>
            {finalVotes.length > 1 && shownVote && (
              <div className="mt-4">
                <FinalVoteSwitch votes={finalVotes} selected={shownVote.scrutin_uid} onSelect={setChosenVote} />
              </div>
            )}
            <div
              aria-hidden="true"
              className={`mt-4 grid border-b border-line pb-1.5 text-xs font-semibold text-ink-faint ${GROUP_TABLE_COLS}`}
            >
              <span />
              <span>Groupe</span>
              <span className="text-right">Amend.</span>
              <span className="truncate">{shownVote ? finalVoteLabel(shownVote) : 'Vote final'}</span>
              <span />
            </div>
            <div>
              {active.map((group) => (
                <GroupRow
                  key={group.uid}
                  group={group}
                  shownVote={shownVote?.scrutin_uid ?? null}
                  compared={selectedUids.includes(group.uid)}
                  compareDisabled={full && !selectedUids.includes(group.uid)}
                  onCompare={() => toggleGroup(group.uid)}
                />
              ))}
            </div>
          </>
        ) : (
          <p className="mt-4 text-sm text-ink-soft">
            Aucun groupe ne porte de vote final ni d'amendement publié sur ce texte.
          </p>
        )}

        {quiet.length > 0 && (
          <div className="mt-5 flex flex-wrap items-center gap-x-2 gap-y-[18px] text-[13px] max-md:gap-y-[18px] md:gap-y-2">
            <span className="mr-1 text-ink-faint">Aucun amendement ni vote final publié :</span>
            {quiet.map((group) => {
              const isSelected = selectedUids.includes(group.uid)
              return (
                <button
                  key={group.uid}
                  type="button"
                  onClick={() => toggleGroup(group.uid)}
                  aria-pressed={isSelected}
                  disabled={full && !isSelected}
                  title={`${group.label} — ajouter à la comparaison`}
                  className={`relative inline-flex h-[26px] items-center gap-1.5 rounded-full pr-2.5 pl-2 text-[13px] font-semibold transition-colors before:absolute before:inset-x-0 before:-inset-y-[9px] disabled:cursor-not-allowed disabled:opacity-40 md:before:hidden ${
                    isSelected ? 'bg-ink text-canvas' : 'bg-surface text-ink-soft hover:bg-surface-soft hover:text-ink'
                  }`}
                >
                  <GroupDot color={group.color} />
                  {group.abbrev}
                </button>
              )
            })}
          </div>
        )}

        <p className="mt-6 hidden max-w-[640px] text-[13px] leading-relaxed text-ink-faint md:block">{data.notes.join(' ')}</p>
      </section>
    </div>
  )
}
