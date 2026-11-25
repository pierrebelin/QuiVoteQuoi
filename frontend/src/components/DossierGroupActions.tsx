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

/**
 * Vote final du groupe, en ligne : lecture, décompte et barre (refonte A). La
 * barre se normalise sur son propre scrutin : aucune échelle commune entre
 * groupes (README.md §6). La position majoritaire publiée peut contredire ce
 * décompte : elle ne sert pas de résumé, le détail du groupe l'affiche à part
 * quand elle en diffère.
 */
function FinalVoteLine({ vote }: { vote: GroupActionsDto['final_votes'][number] }) {
  const { votes_for, votes_against, abstentions, not_voting } = vote.tally
  return (
    <li title={`Scrutin n° ${vote.number} du ${formatDate(vote.date)}`} className="flex flex-col gap-1.5">
      <span className="text-[13px] text-ink-soft">Vote final, {vote.reading ?? formatDate(vote.date)} :</span>
      {/* En bloc : sur téléphone, le décompte passe à la ligne d'un seul tenant. */}
      <span className="inline-block text-[13px] leading-snug text-ink-soft tabular-nums">
        <b className="font-semibold text-yes">{votes_for}</b> pour ·{' '}
        <b className="font-semibold text-no">{votes_against}</b> contre ·{' '}
        <b className="font-semibold text-abstain">{abstentions}</b> abst.
        {not_voting > 0 && ` · ${not_voting} non-votant${not_voting > 1 ? 's' : ''}`}
      </span>
      <span aria-hidden="true" className="block">
        <VoteBar tally={vote.tally} />
      </span>
    </li>
  )
}

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
    <details className="group/row border-b border-line">
      <summary className="grid cursor-pointer list-none gap-x-10 gap-y-4 py-5 lg:grid-cols-2 [&::-webkit-details-marker]:hidden">
        <span className="flex min-w-0 items-center gap-3 lg:col-start-1 lg:row-start-1">
          <PoliticalLogo
            logo={GROUP_LOGOS[group.abbrev]}
            fallback={group.abbrev}
            color={group.color}
            className="h-9 w-14"
          />
          <span className="min-w-0">
            <span className="block text-base leading-tight font-semibold text-ink" title={group.label}>
              {group.label}
            </span>
            <span className="block text-[13px] text-ink-faint">{group.abbrev}</span>
          </span>
        </span>

        {(group.amendment_count > 0 || fates.length > 0) && (
          <span className="flex min-w-0 flex-col gap-3 lg:col-start-1 lg:row-start-2">
            {group.amendment_count > 0 && (
              <span className="text-[15px] text-ink">
                <span className="figure text-xl tabular-nums">{group.amendment_count.toLocaleString('fr-FR')}</span>{' '}
                <span className="text-ink-soft">amendement{group.amendment_count > 1 ? 's' : ''}</span>
              </span>
            )}
            {fates.length > 0 && <FateStats fates={fates} />}
          </span>
        )}

        {group.final_votes.length > 0 && (
          <ul className="flex min-w-0 flex-col gap-3 lg:col-start-2 lg:row-start-2">
            {group.final_votes.map((vote) => (
              <FinalVoteLine key={vote.scrutin_uid} vote={vote} />
            ))}
          </ul>
        )}

        <span className="flex items-center justify-between gap-4 lg:col-start-2 lg:row-start-1 lg:justify-end lg:gap-6">
          <CompareToggle checked={compared} disabled={compareDisabled} onChange={onCompare} />
          <span className="inline-flex min-h-11 items-center gap-1.5 text-sm font-semibold text-ink">
            <span className="group-open/row:hidden">Détails</span>
            <span className="hidden group-open/row:inline">Replier</span>
            <Icon name="down" className="size-[18px] text-ink-faint transition-transform group-open/row:rotate-180" strokeWidth={2} />
          </span>
        </span>
      </summary>
      <div className="mb-5 rounded bg-surface p-4 md:p-5">
        <GroupDetails group={group} />
        {group.amendment_count > 0 && (
          <Link to={amendmentsHref(group)} className="mt-3 inline-block text-sm font-semibold link">
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
            <p>Cochez jusqu'à quatre groupes pour mettre côte à côte leurs synthèses et leurs votes finaux.</p>
            <p>{data.notes.join(' ')}</p>
          </InfoTip>
        </h3>
        <p className="mt-2 hidden max-w-[640px] text-sm leading-relaxed text-ink-soft md:block">
          Cochez jusqu'à quatre groupes pour mettre côte à côte leurs synthèses et leurs votes finaux.
        </p>

        {active.length > 0 ? (
          <div className="mt-5 border-t border-line">
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
