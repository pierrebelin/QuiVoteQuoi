import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { FamilyBadges } from '../components/ThemeBadges'
import {
  Button,
  Card,
  ErrorPanel,
  Loading,
  Note,
  PageHeader,
  Pill,
} from '../components/ui'
import { PoliticalLogo } from '../components/PoliticalLogo'
import { GROUP_LOGOS } from '../content/logos'
import { GroupDot } from './GroupListPage'
import { fetchJson } from '../lib/fetchJson'
import type {
  FinalVoteDto,
  FinalVoteListResponse,
  GroupDto,
  StanceDto,
} from '../types/finalVotes'
import type { FamiliesResponse } from '../types/themes'
import { formatDate } from '../types/scrutins'

const PAGE_SIZE = 20

/** Plafond supposé tant que la réponse n'a pas donné le sien. */
const MAX_GROUPS_BEFORE_LOAD = 4

/**
 * Couleurs des positions, jamais des groupes : le vert et le rouge disent
 * « pour » et « contre », pas « bien » et « mal ». La couleur du groupe reste
 * cantonnée à sa pastille d'identité.
 */
const POSITION_CLASSES: Record<string, string> = {
  for: 'text-yes',
  against: 'text-no',
  abstention: 'text-abstain',
}


/**
 * Colonnes des cartes de position. Au-delà de deux groupes, la largeur du site
 * permet de les aligner de front sans que les chiffres bruts n'aient à céder la
 * place aux seuls pourcentages (README.md §6).
 */
function stanceGridCols(count: number) {
  if (count >= 4) return 'sm:grid-cols-2 lg:grid-cols-4'
  if (count === 3) return 'sm:grid-cols-3'
  if (count === 2) return 'sm:grid-cols-2'
  return ''
}

/** Barre des trois parts. Elle rend visible ce que le pourcentage résume. */
function ShareBar({ share }: { share: NonNullable<StanceDto['share']> }) {
  return (
    <div className="flex h-1.5 w-full overflow-hidden rounded-full bg-surface-soft">
      <div className="bg-yes-bar" style={{ width: `${share.for_percent}%` }} />
      <div className="bg-no-bar" style={{ width: `${share.against_percent}%` }} />
      <div className="bg-abstain-bar" style={{ width: `${share.abstention_percent}%` }} />
    </div>
  )
}

/**
 * Position d'un groupe sur un vote. Le pourcentage ne remplace jamais les
 * chiffres bruts : les deux sont affichés côte à côte (README.md §6).
 */
/** Logo du groupe quand il en a un ; sinon la pastille, le sigle suivant déjà. */
function GroupMark({ abbrev, color, className }: { abbrev: string; color: string | null; className: string }) {
  const logo = GROUP_LOGOS[abbrev]
  if (!logo) return <GroupDot color={color} />
  return <PoliticalLogo logo={logo} fallback={abbrev} color={color} className={className} />
}

function StanceCard({ stance }: { stance: StanceDto }) {
  const { share, tally } = stance
  // Le pourcentage porte la position réellement majoritaire chez les votants,
  // pas la position publiée : sur le scrutin 8431, la source annonce « contre »
  // pour un groupe dont 87 % des votants s'abstiennent.
  const leading = share?.leading_label ?? null
  const divergent =
    stance.majority !== null && share?.leading !== null && stance.majority !== share?.leading

  return (
    <div className="min-w-0 rounded-lg border border-line bg-canvas px-3 py-2.5">
      <div className="flex items-center gap-1.5">
        <GroupMark abbrev={stance.abbrev} color={stance.color} className="h-6 w-10 rounded-md p-0.5" />
        <span className="text-sm font-semibold">{stance.abbrev}</span>
        {leading ? (
          <span
            className={`ml-auto text-sm font-semibold ${POSITION_CLASSES[share?.leading ?? ''] ?? 'text-ink-soft'}`}
          >
            {leading}
            {share?.leading_percent !== null && (
              <span> {share?.leading_percent} %</span>
            )}
          </span>
        ) : (
          // Égalité stricte entre deux positions : aucune ne l'emporte, et en
          // désigner une au hasard serait une invention.
          share !== null && (
            <span className="ml-auto text-sm text-ink-soft">
              {share.tied_labels.join(' et ')} à égalité
            </span>
          )
        )}
      </div>

      {share === null ? (
        <p className="mt-1.5 text-xs text-ink-faint">
          Aucun membre du groupe ne s'est prononcé sur ce vote.
        </p>
      ) : (
        <>
          <div className="mt-2">
            <ShareBar share={share} />
          </div>
          <p className="mt-1.5 text-xs text-ink-soft">
            <span className="font-semibold text-yes">{tally.votes_for}</span> pour ·{' '}
            <span className="font-semibold text-no">{tally.votes_against}</span> contre ·{' '}
            <span className="font-semibold text-abstain">{tally.abstentions}</span> abst.
            <span className="text-ink-faint">
              {' '}
              — sur {share.voters} votant{share.voters > 1 ? 's' : ''}
            </span>
          </p>
        </>
      )}

      {divergent && stance.majority_label && (
        <p className="mt-1 text-xs text-ink-faint">
          Position majoritaire publiée par l'Assemblée : {stance.majority_label}.
        </p>
      )}

      {(tally.not_voting > 0 || tally.voluntary_not_voting > 0) && (
        <p className="mt-0.5 text-xs text-ink-faint">
          {tally.not_voting > 0 && `${tally.not_voting} non-votant${tally.not_voting > 1 ? 's' : ''}`}
          {tally.not_voting > 0 && tally.voluntary_not_voting > 0 && ' · '}
          {tally.voluntary_not_voting > 0 &&
            `${tally.voluntary_not_voting} non-votant${tally.voluntary_not_voting > 1 ? 's' : ''} volontaire${tally.voluntary_not_voting > 1 ? 's' : ''}`}
        </p>
      )}
    </div>
  )
}

/**
 * Un groupe sélectionné mais absent du scrutin : la lacune est nommée, jamais
 * comblée par un zéro (README.md §2).
 */
function MissingStance({ group }: { group: GroupDto }) {
  return (
    <div className="min-w-0 rounded-lg border border-dashed border-line px-3 py-2.5">
      <div className="flex items-center gap-1.5">
        <GroupMark abbrev={group.abbrev} color={group.color} className="h-6 w-10 rounded-md p-0.5 opacity-60" />
        <span className="text-sm font-semibold text-ink-soft">{group.abbrev}</span>
      </div>
      <p className="mt-1.5 text-xs text-ink-faint">
        Aucune ligne pour ce groupe sur ce scrutin.
      </p>
    </div>
  )
}

function VoteRow({ vote, selected }: { vote: FinalVoteDto; selected: GroupDto[] }) {
  return (
    <Card className="px-4 py-3.5">
      {/* Sur petit écran la date passe sous le titre : gardée sur la même
          ligne, elle réduisait le titre à trois mots par ligne. */}
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <Pill tone={vote.adopted ? 'yes' : 'no'}>
          {vote.adopted ? 'adopté' : 'rejeté'}
        </Pill>
        <h3 className="min-w-0 flex-1 basis-full text-[15px] font-semibold leading-snug sm:basis-auto">
          <Link
            to={`/textes/${encodeURIComponent(vote.text_key)}`}
            className="transition-colors hover:text-accent"
          >
            {vote.text_label}
          </Link>
        </h3>
        <span className="shrink-0 text-xs text-ink-faint">
          {formatDate(vote.date)} · n° {vote.number}
        </span>
      </div>

      <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-ink-faint">
        <span>{vote.ballot_type_label}</span>
        {vote.reading && <span>· {vote.reading}</span>}
        <FamilyBadges families={vote.families} />
        {vote.dossier_uid && (
          <Link
            to={`/dossiers/${vote.dossier_uid}`}
            className="text-accent underline"
          >
            Dossier
          </Link>
        )}
        <Link to={`/scrutins/${vote.scrutin_uid}`} className="text-accent underline">
          Détail du scrutin
        </Link>
      </div>

      {vote.official_text ? (
        <div className="mt-2 rounded-md bg-surface-soft px-3 py-2 text-xs text-ink-soft">
          <p>
            Version liée au vote :{' '}
            <a
              href={vote.official_text.official_url}
              target="_blank"
              rel="noreferrer"
              className="font-medium text-accent underline"
            >
              {vote.official_text.version_label}
            </a>{' '}
            <span className="text-ink-faint">· {vote.official_text.document_uid}</span>
          </p>
          <p className="mt-1 text-ink-faint">
            Référence établie par {vote.official_text.source_producer} · licence :{' '}
            {vote.official_text.source_license} ·{' '}
            <a
              href={vote.official_text.mapping_source_url}
              target="_blank"
              rel="noreferrer"
              className="text-accent underline"
            >
              voir la source du rattachement
            </a>
          </p>
        </div>
      ) : (
        <p className="mt-2 text-xs text-ink-faint">
          La version exacte du texte lié à ce vote n’est pas encore référencée.
        </p>
      )}

      {selected.length > 0 && (
        <div className={`mt-2 grid gap-2 ${stanceGridCols(selected.length)}`}>
          {selected.map((group) => {
            const stance = vote.stances.find((s) => s.group_uid === group.uid)
            return stance ? (
              <StanceCard key={group.uid} stance={stance} />
            ) : (
              <MissingStance key={group.uid} group={group} />
            )
          })}
        </div>
      )}

      <p className="mt-2 text-xs text-ink-faint">
        Assemblée entière : {vote.synthesis.votes_for} pour ·{' '}
        {vote.synthesis.votes_against} contre · {vote.synthesis.abstentions} abst.
      </p>
    </Card>
  )
}

/**
 * Sélecteur des groupes comparés. Une pastille par groupe du référentiel :
 * l'offre reste entière et visible, et la limite est annoncée avant d'être
 * atteinte plutôt que signalée par une erreur du serveur.
 *
 * Le renvoi vers les fiches de groupes est posé ici plutôt que dans la barre de
 * navigation : c'est en choisissant les sigles qu'on se demande qui ils
 * recouvrent, pas trois pages plus tôt.
 */
function GroupPicker({
  groups,
  selected,
  max,
  onToggle,
}: {
  groups: GroupDto[]
  selected: string[]
  max: number
  onToggle: (abbrev: string) => void
}) {
  const full = selected.length >= max

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
      <span className="text-xs font-medium text-ink-faint">
        Groupes comparés {selected.length}/{max}
      </span>
      <div className="flex flex-wrap gap-1.5">
        {groups.map((group) => {
          const isSelected = selected.includes(group.abbrev)
          return (
            <button
              key={group.uid}
              type="button"
              onClick={() => onToggle(group.abbrev)}
              aria-pressed={isSelected}
              disabled={full && !isSelected}
              title={group.label}
              className={`flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-[13px] font-medium ring-1 ring-inset transition-colors has-[img]:pl-1 ${
                isSelected
                  ? 'bg-ink text-white ring-ink'
                  : 'bg-surface text-ink-soft shadow-card ring-line hover:text-ink hover:ring-line-strong disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:ring-line'
              }`}
            >
              <GroupMark abbrev={group.abbrev} color={group.color} className="h-6 w-9 rounded-md p-0.5" />
              {group.abbrev}
            </button>
          )
        })}
      </div>
      {groups.length > 0 && (
        <Link
          to="/groupes"
          className="text-xs font-medium text-accent hover:underline"
        >
          Qui est qui : effectifs et composition des {groups.length} groupes →
        </Link>
      )}
    </div>
  )
}

/**
 * Votes sur l'ensemble d'un texte, groupe par groupe.
 *
 * Le groupe parlementaire est l'unité affichée, jamais le parti : ce ne sont
 * pas les mêmes objets, et une équivalence approximative présentée comme un
 * fait serait une fausse information (README.md §3.1).
 */
export default function GroupVotesPage() {
  const [params, setParams] = useSearchParams()

  // Aucune sélection par défaut : présélectionner des groupes reviendrait à
  // désigner la comparaison qui mérite d'être faite, et à orienter la lecture
  // avant qu'elle commence (README.md §6).
  const groupsParam = params.get('groupes') ?? ''
  const theme = params.get('theme') ?? ''
  const offset = Math.max(0, Number(params.get('offset') ?? 0) || 0)

  const requested = groupsParam
    .split(',')
    .map((token) => token.trim())
    .filter(Boolean)

  // Sans groupe demandé, la page n'affiche aucun vote : la requête ne sert
  // alors qu'à ramener le référentiel des groupes du sélecteur, et une page
  // entière de votes serait chargée pour rien.
  const query = new URLSearchParams({
    groupes: groupsParam,
    limit: String(requested.length > 0 ? PAGE_SIZE : 1),
    offset: String(offset),
  })
  if (theme) query.set('theme', theme)

  const { data, isLoading, isError, error, isPlaceholderData } =
    useQuery<FinalVoteListResponse>({
      queryKey: ['votes-finaux', query.toString()],
      queryFn: () =>
        fetchJson<FinalVoteListResponse>(`/api/votes-finaux?${query}`),
      // Changer de groupes ne doit pas vider le haut de la page : le référentiel
      // du sélecteur voyage dans la même réponse, et le laisser disparaître
      // ferait clignoter les pastilles au moment même où on clique dessus. La
      // réponse précédente tient le décor pendant que les votes se rechargent.
      placeholderData: keepPreviousData,
      retry: false,
    })

  const families = useQuery<FamiliesResponse>({
    queryKey: ['themes'],
    queryFn: () =>
      fetchJson<FamiliesResponse>('/api/themes'),
    retry: false,
  })

  function update(next: Record<string, string>) {
    const merged = new URLSearchParams(params)
    for (const [key, value] of Object.entries(next)) {
      if (value) merged.set(key, value)
      else merged.delete(key)
    }
    // Tout changement de filtre ramène en tête de liste : garder l'ancien
    // décalage afficherait une page vide sans le dire. La pagination, elle,
    // pose son propre décalage et n'est pas concernée.
    if (!('offset' in next)) merged.delete('offset')
    setParams(merged)
  }

  const groups = data?.groups ?? []
  const selected = data?.selected ?? []
  const shown = data?.items.length ?? 0
  const maxGroups = data?.max_compared_groups ?? MAX_GROUPS_BEFORE_LOAD

  // Le sigle affiché fait foi : un groupe renommé est proposé sous son sigle
  // courant, et c'est celui-là qui part dans l'adresse.
  // Pendant un rechargement, la réponse en main porte encore l'ancienne
  // sélection : c'est l'adresse qui fait foi, sinon la pastille qu'on vient de
  // cliquer resterait éteinte jusqu'à l'arrivée des votes.
  const selectedAbbrevs =
    data === undefined || isPlaceholderData
      ? requested
      : selected.map((group) => group.abbrev)
  const hasSelection = selectedAbbrevs.length > 0
  // Les votes affichés ne correspondent plus à la sélection : la zone se
  // recharge, le reste de la page ne bouge pas.
  const reloading = hasSelection && (isLoading || isPlaceholderData) && !isError

  function toggleGroup(abbrev: string) {
    const next = selectedAbbrevs.includes(abbrev)
      ? selectedAbbrevs.filter((token) => token !== abbrev)
      : [...selectedAbbrevs, abbrev].slice(0, maxGroups)
    // Retirer le dernier groupe ramène à l'état de départ : l'adresse perd le
    // paramètre, et la page repose la question au lieu de retomber sur une
    // sélection qu'elle aurait choisie seule.
    update({ groupes: next.join(',') })
  }

  const totalUnfiltered = data?.total_unfiltered ?? 0
  const missingGroups = (isPlaceholderData || !data ? [] : selected).filter(
    (group) => group.final_vote_count < totalUnfiltered,
  )

  return (
    <>
      <PageHeader
        title="Comparer les votes des groupes"
        lede="Choisissez les groupes à mettre côte à côte : chaque vote sur l'ensemble d'un texte affiche leur position, en pourcentage et en voix."
        aside={
          <label className="flex items-center gap-2 text-xs text-ink-faint">
            Thème
            <select
              value={theme}
              onChange={(e) => update({ theme: e.target.value })}
              className="rounded-lg border border-line bg-surface px-2.5 py-1.5 text-sm text-ink shadow-card focus:border-accent focus:ring-2 focus:ring-accent/15 focus:outline-none"
            >
              <option value="">Tous</option>
              {(families.data?.families ?? []).map((family) => (
                <option key={family.code} value={family.code}>
                  {family.label}
                </option>
              ))}
            </select>
          </label>
        }
      />

      <p className="mb-4 text-xs text-ink-faint">
        <Link to="/votes-par-groupe/methode" className="text-accent underline">
          Comment sont retenus les votes sur l’ensemble ?
        </Link>
      </p>

      <div className="mb-4 space-y-3">
        {data ? (
          <GroupPicker
            groups={groups}
            selected={selectedAbbrevs}
            max={maxGroups}
            onToggle={toggleGroup}
          />
        ) : isLoading ? (
          <Card className="px-4 py-3">
            <Loading>Chargement des groupes disponibles…</Loading>
          </Card>
        ) : null}

        {/*
          Les notes de lecture sont dues (README.md §6) mais tiennent dans un
          seul encadré : quatre blocs empilés repoussaient les votes hors de
          l'écran. Le contenu est intégral, c'est le poids visuel qui baisse.
        */}
        {data && hasSelection && (
          <Note>
            <div className="space-y-1">
              <p>
                {data.scope_note}{' '}
                <Link to="/scrutins" className="text-accent underline">
                  Voir tous les scrutins
                </Link>
              </p>
              <p>
                {data.share_note} {data.outcome_note}
              </p>
              {missingGroups.map((group) => (
                <p key={group.uid}>
                  {group.label} ({group.abbrev}) n'apparaît que sur{' '}
                  {group.final_vote_count} des {data.total_unfiltered} votes sur
                  l'ensemble : le groupe n'existait pas sur les autres.
                </p>
              ))}
              {theme && (
                <p>
                  La thématisation est en cours : {data.total_with_family} des{' '}
                  {data.total_unfiltered} votes sur l'ensemble portent une
                  famille. Filtrer par thème laisse donc de côté les textes pas
                  encore rattachés.{' '}
                  <Link to="/themes/methode" className="text-accent underline">
                    Méthode
                  </Link>
                </p>
              )}
            </div>
          </Note>
        )}
      </div>

      {!hasSelection && data && (
        <Card className="px-4 py-3">
          <p className="text-sm font-semibold">
            Aucun groupe n'est sélectionné au départ.
          </p>
          <p className="mt-1 max-w-3xl text-sm leading-relaxed text-ink-soft">
            Choisir pour vous les groupes à comparer reviendrait à désigner la
            comparaison qui mérite d'être faite. Sélectionnez ci-dessus les
            groupes que vous voulez lire côte à côte — {maxGroups} au maximum —
            pour afficher les votes sur l'ensemble.
          </p>
          <p className="mt-2 max-w-3xl text-sm leading-relaxed text-ink-soft">
            Les sigles ne vous parlent pas ? La{' '}
            <Link to="/groupes" className="text-accent underline">
              liste des groupes
            </Link>{' '}
            donne pour chacun son intitulé complet, son effectif et ses dates
            d'existence. Un groupe parlementaire n'est pas un parti :{' '}
            <Link to="/comprendre#q-groupe" className="text-accent underline">
              pourquoi cette distinction
            </Link>
            .
          </p>
        </Card>
      )}

      {reloading && <Loading>Chargement des votes…</Loading>}
      {isError && <ErrorPanel error={error} />}

      {data && hasSelection && !reloading && (
        <>
          <p className="mb-2 text-xs text-ink-faint">
            {data.total.toLocaleString('fr-FR')} vote{data.total > 1 ? 's' : ''} sur
            l'ensemble
            {data.total > 0 && ` — affichés ${data.offset + 1} à ${data.offset + shown}`}
          </p>

          {shown === 0 ? (
            <Card className="px-4 py-3">
              <p className="text-sm text-ink-soft">
                Aucun vote sur l'ensemble ne correspond à ce filtre.
              </p>
            </Card>
          ) : (
            <div className="space-y-2.5">
              {data.items.map((vote) => (
                <VoteRow key={vote.scrutin_uid} vote={vote} selected={selected} />
              ))}
            </div>
          )}

          <div className="mt-5 flex items-center justify-between">
            <Button
              onClick={() =>
                update({ offset: String(Math.max(0, offset - PAGE_SIZE)) })
              }
              disabled={offset === 0}
            >
              ← Précédents
            </Button>
            <Button
              onClick={() => update({ offset: String(offset + PAGE_SIZE) })}
              disabled={offset + shown >= data.total}
            >
              Suivants →
            </Button>
          </div>
        </>
      )}
    </>
  )
}
