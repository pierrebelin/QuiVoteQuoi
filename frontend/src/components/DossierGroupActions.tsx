import { useQuery } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { GroupDot } from '../pages/GroupListPage'
import { GROUP_LOGOS } from '../content/logos'
import { InfoTip } from './InfoTip'
import { PoliticalLogo } from './PoliticalLogo'
import { Card, ErrorPanel, Pill, TallyLine, VoteBar } from './ui'
import { leadingPosition, positionLabels } from '../types/scrutins'
import type {
  DossierGroupActionsResponse,
  FateCountDto,
  GroupActionsDto,
  SummarySourceDto,
} from '../types/dossierGroupActions'

const MAX_COMPARED_GROUPS = 4

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
    <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-xs">
      <span className="text-ink-faint">Sources :</span>
      {sources.map((source) =>
        source.official_url ? (
          <a
            key={source.id}
            href={source.official_url}
            target="_blank"
            rel="noreferrer"
            className="text-accent hover:underline"
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
 * Sort d'amendements, avec le libellé de la source. Le vert et le rouge disent
 * le sort, jamais le groupe : la couleur du groupe n'est pas posée ici.
 */
function FateChip({ fate }: { fate: FateCountDto }) {
  const label = fate.label.toLowerCase()
  const tone = label.startsWith('adopt')
    ? 'bg-yes-soft text-yes'
    : label.startsWith('rejet')
      ? 'bg-no-soft text-no'
      : 'bg-surface-soft text-ink-soft ring-1 ring-inset ring-line'
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-semibold ${tone}`}>
      {fate.label || 'Sort non publié'}
      <span className="tabular-nums">{fate.count.toLocaleString('fr-FR')}</span>
    </span>
  )
}

type Position = keyof typeof positionLabels

function isPosition(value: string): value is Position {
  return value in positionLabels
}

function positionLabel(value: string) {
  return isPosition(value) ? positionLabels[value] : value
}

/**
 * Vote final du groupe : lecture et décompte, sans couleur. La position
 * majoritaire publiée peut contredire ce décompte : elle ne sert pas de résumé,
 * le détail du groupe l'affiche à part quand elle en diffère.
 */
function FinalVoteChip({ vote }: { vote: GroupActionsDto['final_votes'][number] }) {
  const { votes_for, votes_against, abstentions, not_voting } = vote.tally
  return (
    <span
      title={`Scrutin n° ${vote.number} du ${formatDate(vote.date)}`}
      className="rounded-md bg-surface-soft px-2 py-0.5 text-xs font-medium text-ink-soft ring-1 ring-inset ring-line"
    >
      Vote final, {vote.reading ?? formatDate(vote.date)} :{' '}
      {/* En bloc : sur téléphone, le décompte passe à la ligne d'un seul tenant. */}
      <span className="inline-block">
        {votes_for} pour · {votes_against} contre · {abstentions} abst.
        {not_voting > 0 && ` · ${not_voting} non-votant${not_voting > 1 ? 's' : ''}`}
      </span>
    </span>
  )
}

function FinalVote({ vote }: { vote: GroupActionsDto['final_votes'][number] }) {
  return (
    <div className="rounded-lg border border-line bg-surface px-3 py-2">
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <Link to={`/scrutins/${vote.scrutin_uid}`} className="text-sm font-medium text-accent hover:underline">
          Scrutin n° {vote.number}
        </Link>
        <span className="text-xs text-ink-faint">{formatDate(vote.date)}</span>
        {vote.reading && <span className="text-xs text-ink-soft">· {vote.reading}</span>}
        <Pill tone={vote.outcome_code === 'adopted' ? 'yes' : vote.outcome_code === 'rejected' ? 'no' : 'neutral'}>
          {vote.outcome_label}
        </Pill>
      </div>
      <p className="mt-1 text-xs leading-relaxed text-ink-soft">{vote.text_label}</p>
      <div className="mt-2">
        <VoteBar tally={vote.tally} />
      </div>
      <div className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1">
        <TallyLine tally={vote.tally} />
        {vote.majority_position && vote.majority_position !== leadingPosition(vote.tally) && (
          <span className="text-xs text-ink-soft">
            Position majoritaire publiée par l'Assemblée : {positionLabel(vote.majority_position).toLowerCase()}
          </span>
        )}
        <a href={vote.official_url} target="_blank" rel="noreferrer" className="text-xs text-accent hover:underline">
          Source officielle ↗
        </a>
      </div>
    </div>
  )
}

/** Synthèse et votes finaux d'un groupe, tels que publiés. */
function GroupDetails({ group }: { group: GroupActionsDto }) {
  return (
    <div className="space-y-3">
      {(group.start_date || group.end_date) && (
        <p className="text-xs text-ink-faint">
          Groupe concerné du {group.start_date ? formatDate(group.start_date) : 'début de période'} au{' '}
          {group.end_date ? formatDate(group.end_date) : '—'}
        </p>
      )}
      {group.summary ? (
        <div className="rounded-lg bg-surface-soft px-3 py-2">
          <Pill tone="info">{group.summary.label}</Pill>
          <p className="mt-1.5 text-sm leading-relaxed text-ink-soft">{group.summary.text}</p>
          <SourceLinks sources={group.summary.sources} />
        </div>
      ) : group.state === 'summary_pending' ? (
        <p className="text-xs text-ink-faint">
          Synthèse automatique en attente de génération. Les actes restent consultables.
        </p>
      ) : group.state === 'summary_unavailable' ? (
        <p className="text-xs text-ink-faint">
          Synthèse automatique indisponible pour le moment. Les actes restent consultables.
        </p>
      ) : null}
      <div>
        <p className="mb-1.5 text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
          Votes finaux, par lecture
        </p>
        {group.final_votes.length === 0 ? (
          <p className="text-xs text-ink-faint">Aucun vote final publié.</p>
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
      className={`inline-flex shrink-0 items-center gap-1.5 text-xs text-ink-soft ${disabled ? 'cursor-not-allowed opacity-40' : 'cursor-pointer hover:text-ink'}`}
    >
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={onChange}
        className="size-3.5 accent-ink"
      />
      Comparer
    </label>
  )
}

/**
 * Ligne d'un groupe qui a agi sur le texte : son logo, ses amendements par sort,
 * ses votes finaux. Les groupes restent dans l'ordre de la source : aucun
 * classement, aucune échelle commune entre groupes (README.md §6).
 */
function GroupRow({
  group,
  compared,
  compareDisabled,
  onCompare,
}: {
  group: GroupActionsDto
  compared: boolean
  compareDisabled: boolean
  onCompare: () => void
}) {
  const fates = group.amendment_fates ?? []
  return (
    <details className="group/row border-t border-line first:border-t-0">
      <summary className="flex cursor-pointer list-none flex-wrap items-center gap-x-4 gap-y-2 py-3 [&::-webkit-details-marker]:hidden">
        <span className="flex min-w-0 flex-1 basis-56 items-center gap-3">
          <PoliticalLogo
            logo={GROUP_LOGOS[group.abbrev]}
            fallback={group.abbrev}
            color={group.color}
            className="h-9 w-14"
          />
          <span className="min-w-0">
            <span className="block truncate text-sm font-semibold text-ink" title={group.label}>
              {group.label}
            </span>
            <span className="block text-xs text-ink-faint">{group.abbrev}</span>
          </span>
        </span>

        <span className="flex flex-1 basis-64 flex-wrap items-center gap-1.5">
          {group.amendment_count > 0 && (
            <span className="mr-1 text-sm text-ink">
              <span className="font-semibold tabular-nums">{group.amendment_count.toLocaleString('fr-FR')}</span>{' '}
              <span className="text-ink-soft">amendement{group.amendment_count > 1 ? 's' : ''}</span>
            </span>
          )}
          {fates.map((fate) => (
            <FateChip key={fate.label} fate={fate} />
          ))}
          {group.final_votes.map((vote) => (
            <FinalVoteChip key={vote.scrutin_uid} vote={vote} />
          ))}
        </span>

        <span className="flex items-center gap-4">
          <CompareToggle checked={compared} disabled={compareDisabled} onChange={onCompare} />
          <span className="text-xs font-medium text-accent group-open/row:hidden">Détails</span>
          <span className="hidden text-xs font-medium text-accent group-open/row:inline">Replier</span>
        </span>
      </summary>
      <div className="mb-3 rounded-xl border border-line bg-surface-soft/50 px-4 py-3">
        <GroupDetails group={group} />
        {group.amendment_count > 0 && (
          <Link to={amendmentsHref(group)} className="mt-3 inline-block text-sm font-medium text-accent hover:underline">
            Voir ses {group.amendment_count.toLocaleString('fr-FR')} amendement
            {group.amendment_count > 1 ? 's' : ''} →
          </Link>
        )}
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
          <h4 className="truncate text-sm font-semibold text-ink" title={group.label}>
            {group.label}
          </h4>
          <p className="text-xs text-ink-faint">{group.abbrev}</p>
        </div>
        <button
          type="button"
          onClick={onRemove}
          aria-label={`Retirer ${group.abbrev} de la comparaison`}
          className="rounded-md px-1.5 text-lg leading-none text-ink-faint hover:bg-surface-soft hover:text-ink"
        >
          ×
        </button>
      </div>
      {hasActions(group) ? (
        <>
          {(group.amendment_fates ?? []).length > 0 && (
            <div className="mb-3 flex flex-wrap gap-1.5">
              {(group.amendment_fates ?? []).map((fate) => (
                <FateChip key={fate.label} fate={fate} />
              ))}
            </div>
          )}
          <GroupDetails group={group} />
        </>
      ) : (
        <p className="text-xs leading-relaxed text-ink-faint">
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

  return (
    <div className="space-y-4">
      {selected.length > 0 && (
        <section>
          <div className="mb-2 flex items-baseline justify-between gap-3">
            <h3 className="text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
              Groupes comparés {selected.length}/{MAX_COMPARED_GROUPS}
            </h3>
            <button
              type="button"
              onClick={() => {
                const updated = new URLSearchParams(params)
                updated.delete('avis_groupes')
                setParams(updated, { replace: true, preventScrollReset: true })
              }}
              className="text-xs font-medium text-accent hover:underline"
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

      <Card className="px-4 py-4 sm:px-6 sm:py-5">
        <h3 className="text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
          Ce que chaque groupe a fait sur ce texte
          <InfoTip label="Comment lire cette section" className="md:hidden">
            <p>Cochez jusqu'à quatre groupes pour mettre côte à côte leurs synthèses et leurs votes finaux.</p>
            <p>{data.notes.join(' ')}</p>
          </InfoTip>
        </h3>
        <p className="mt-1 hidden text-xs text-ink-faint md:block">
          Cochez jusqu'à quatre groupes pour mettre côte à côte leurs synthèses et leurs votes finaux.
        </p>

        {active.length > 0 ? (
          <div className="mt-2">
            {active.map((group) => (
              <GroupRow
                key={group.uid}
                group={group}
                compared={selectedUids.includes(group.uid)}
                compareDisabled={full && !selectedUids.includes(group.uid)}
                onCompare={() => toggleGroup(group.uid)}
              />
            ))}
          </div>
        ) : (
          <p className="mt-3 text-sm text-ink-soft">
            Aucun groupe ne porte de vote final ni d'amendement publié sur ce dossier.
          </p>
        )}

        {quiet.length > 0 && (
          <div className="mt-3 flex flex-wrap items-center gap-1.5 border-t border-line pt-3 text-[13px]">
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
                  className={`inline-flex items-center gap-1.5 rounded-full py-0.5 pr-2.5 pl-1.5 text-xs font-semibold ring-1 ring-inset transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${
                    isSelected ? 'bg-ink text-surface ring-ink' : 'bg-surface text-ink-soft ring-line hover:ring-line-strong'
                  }`}
                >
                  <GroupDot color={group.color} />
                  {group.abbrev}
                </button>
              )
            })}
          </div>
        )}

        <p className="mt-4 hidden text-xs leading-relaxed text-ink-faint md:block">{data.notes.join(' ')}</p>
      </Card>
    </div>
  )
}
