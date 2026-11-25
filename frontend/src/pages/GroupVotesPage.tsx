import { useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { InfoTip } from '../components/InfoTip'
import { BottomSheet } from '../components/BottomSheet'
import { Icon } from '../components/icons'
import { CheckRow, FOCUS, SheetButton } from '../components/SheetControls'
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
  StanceExplanationDto,
} from '../types/finalVotes'
import type { FamiliesResponse } from '../types/themes'
import { formatDate, positionLabels } from '../types/scrutins'

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

/** Même table pour la pastille d'état : pastille ronde de la position, puis le mot. */
const POSITION_TONES: Record<string, 'yes' | 'no' | 'abstain'> = {
  for: 'yes',
  against: 'no',
  abstention: 'abstain',
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
    <div className="bar-grow flex h-1.5 w-full overflow-hidden rounded-full bg-surface-soft">
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
/** Pourquoi rien n'est présélectionné : la neutralité, puis l'aide aux sigles. */
function NoSelectionHelp({ maxGroups }: { maxGroups: number }) {
  return (
    <>
      <p className="max-w-2xl text-sm leading-relaxed text-ink-soft md:text-[15px]">
        Choisir pour vous les groupes à comparer reviendrait à désigner la
        comparaison qui mérite d'être faite. Sélectionnez ci-dessus les
        groupes que vous voulez lire côte à côte — {maxGroups} au maximum —
        pour afficher les votes sur l'ensemble.
      </p>
      <p className="mt-2 max-w-2xl text-sm leading-relaxed text-ink-soft md:text-[15px]">
        Les sigles ne vous parlent pas ? La{' '}
        <Link to="/groupes" className="link">
          liste des groupes
        </Link>{' '}
        donne pour chacun son intitulé complet, son effectif et ses dates
        d'existence. Un groupe parlementaire n'est pas un parti :{' '}
        <Link to="/comprendre#q-groupe" className="link">
          pourquoi cette distinction
        </Link>
        .
      </p>
    </>
  )
}

function GroupMark({ abbrev, color, className }: { abbrev: string; color: string | null; className: string }) {
  const logo = GROUP_LOGOS[abbrev]
  if (!logo) return <GroupDot color={color} />
  return <PoliticalLogo logo={logo} fallback={abbrev} color={color} className={className} />
}

/**
 * Début de l'explication de vote du groupe, repris tel quel du compte rendu
 * (README.md §6) : trois lignes, puis la fiche du scrutin pour la suite. Le
 * vote de l'orateur vient de son vote nominatif, jamais de celui du groupe.
 */
function StanceExplanation({
  explanation,
  scrutinUid,
}: {
  explanation: StanceExplanationDto
  scrutinUid: string
}) {
  const position = explanation.speaker_position
  return (
    <div className="mt-3">
      <p className="text-[13px] text-ink-faint">
        {explanation.speaker_name ?? 'Orateur du groupe'}
        {', '}
        {position ? (
          <span className={`font-medium ${POSITION_CLASSES[position] ?? 'text-ink-soft'}`}>
            a voté {positionLabels[position].toLowerCase()}
          </span>
        ) : (
          'position non publiée'
        )}
      </p>
      {explanation.preview && (
        <p className="mt-1 line-clamp-3 text-sm leading-[1.45] text-ink-soft">
          {/* L'aperçu coupé porte déjà ses points de suspension. */}«{' '}
          {explanation.preview_truncated ? explanation.preview : `${explanation.preview} »`}
        </p>
      )}
      <p className="mt-1.5 text-sm">
        <Link
          to={`/scrutins/${scrutinUid}#explication-${explanation.rank}`}
          className="relative font-semibold link after:absolute after:-inset-x-1 after:-inset-y-3 after:content-['']"
        >
          Lire l’explication en entier
        </Link>
      </p>
    </div>
  )
}

function StanceCard({ stance, vote }: { stance: StanceDto; vote: FinalVoteDto }) {
  const { share, tally } = stance
  // Le pourcentage porte la position réellement majoritaire chez les votants,
  // pas la position publiée : sur le scrutin 8431, la source annonce « contre »
  // pour un groupe dont 87 % des votants s'abstiennent.
  const leading = share?.leading_label ?? null
  const divergent =
    stance.majority !== null && share?.leading !== null && stance.majority !== share?.leading

  return (
    <div className="min-w-0 border-t border-line pt-3.5 pb-1">
      <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1">
        <GroupMark abbrev={stance.abbrev} color={stance.color} className="h-6 w-10 p-0.5" />
        <span className="text-[15px] font-semibold">{stance.abbrev}</span>
        {leading ? (
          <span className="ml-auto inline-flex items-center gap-2">
            {POSITION_TONES[share?.leading ?? ''] ? (
              <Pill tone={POSITION_TONES[share?.leading ?? '']}>{leading}</Pill>
            ) : (
              <span className="text-[13px] font-semibold text-ink-soft">{leading}</span>
            )}
            {share?.leading_percent !== null && (
              <span
                className={`figure text-[22px] ${POSITION_CLASSES[share?.leading ?? ''] ?? 'text-ink-soft'}`}
              >
                {share?.leading_percent} %
              </span>
            )}
          </span>
        ) : (
          // Égalité stricte entre deux positions : aucune ne l'emporte, et en
          // désigner une au hasard serait une invention.
          share !== null && (
            <span className="ml-auto text-[13px] font-semibold text-ink-soft">
              {share.tied_labels.join(' et ')} à égalité
            </span>
          )
        )}
      </div>

      {share === null ? (
        <p className="mt-2 text-[13px] text-ink-faint">
          Aucun membre du groupe ne s'est prononcé sur ce vote.
        </p>
      ) : (
        <>
          <div className="mt-2.5">
            <ShareBar share={share} />
          </div>
          <p className="mt-2 text-[13px] text-ink-soft">
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
        <p className="mt-1 text-[13px] text-ink-faint">
          Position majoritaire publiée par l'Assemblée : {stance.majority_label}.
        </p>
      )}

      {tally.not_voting > 0 && (
        <p className="mt-0.5 text-[13px] text-ink-faint">
          {tally.not_voting} non-votant{tally.not_voting > 1 ? 's' : ''}
        </p>
      )}

      {/* Seul un compte rendu qui porte des explications pour ce vote permet
          de dire qu'un groupe n'en a pas donné. */}
      {vote.explanations_state === 'explained' &&
        (stance.explanation ? (
          <StanceExplanation explanation={stance.explanation} scrutinUid={vote.scrutin_uid} />
        ) : (
          <p className="mt-2.5 text-[13px] text-ink-faint">
            Pas d’explication de vote.
          </p>
        ))}
    </div>
  )
}

/**
 * Un groupe sélectionné mais absent du scrutin : la lacune est nommée, jamais
 * comblée par un zéro (README.md §2).
 */
function MissingStance({ group }: { group: GroupDto }) {
  return (
    <div className="min-w-0 border-t border-dashed border-line-strong pt-3.5 pb-1">
      <div className="flex items-center gap-2.5">
        <GroupMark abbrev={group.abbrev} color={group.color} className="h-6 w-10 p-0.5 opacity-60" />
        <span className="text-[15px] font-semibold text-ink-soft">{group.abbrev}</span>
      </div>
      <p className="mt-2 text-[13px] text-ink-faint">
        Aucune ligne pour ce groupe sur ce vote.
      </p>
    </div>
  )
}

function VoteRow({ vote, selected }: { vote: FinalVoteDto; selected: GroupDto[] }) {
  return (
    <Card className="px-4 py-4 md:px-7 md:py-6">
      {/* L'état et la date sur une ligne, le titre dessous (refonte A) : le
          titre garde toute la largeur, à 375 comme à 1280 px. L'ordre de lecture
          reste état, titre, date. */}
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <span className="order-1">
          <Pill tone={vote.adopted ? 'yes' : 'no'}>
            {vote.adopted ? 'adopté' : 'rejeté'}
          </Pill>
        </span>
        <h3 className="order-3 min-w-0 basis-full heading-5 text-balance text-ink md:text-[19px] md:leading-6">
          <Link
            to={`/textes/${encodeURIComponent(vote.text_key)}`}
            className="transition-colors hover:text-accent"
          >
            {vote.text_label}
          </Link>
        </h3>
        <span className="order-2 ml-auto shrink-0 text-[13px] text-ink-faint">
          {formatDate(vote.date)} · n° {vote.number}
        </span>
      </div>

      <div className="mt-1.5 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-[13px] text-ink-faint">
        <span>
          {vote.ballot_type_label}
          {vote.reading && ` · ${vote.reading}`}
        </span>
        <FamilyBadges families={vote.families} onCard />
        {vote.dossier_uid && (
          <Link
            to={`/dossiers/${vote.dossier_uid}`}
            className="relative text-[15px] font-semibold link after:absolute after:-inset-x-1 after:-inset-y-3 after:content-['']"
          >
            Texte
          </Link>
        )}
        <Link
          to={`/scrutins/${vote.scrutin_uid}`}
          className="relative text-[15px] font-semibold link after:absolute after:-inset-x-1 after:-inset-y-3 after:content-['']"
        >
          Détail du vote
        </Link>
      </div>

      {vote.official_text ? (
        <div className="mt-3 border-t border-line pt-3 text-[13px] leading-relaxed text-ink-soft">
          <p>
            Version liée au vote :{' '}
            <a
              href={vote.official_text.official_url}
              target="_blank"
              rel="noreferrer"
              className="font-semibold link"
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
              className="link"
            >
              voir la source du rattachement
            </a>
          </p>
        </div>
      ) : (
        <p className="mt-3 text-[13px] text-ink-faint">
          La version exacte du texte lié à ce vote n’est pas encore référencée.
        </p>
      )}

      {selected.length > 0 && (
        <div className={`mt-3 grid gap-x-8 ${stanceGridCols(selected.length)}`}>
          {selected.map((group) => {
            const stance = vote.stances.find((s) => s.group_uid === group.uid)
            return stance ? (
              <StanceCard key={group.uid} stance={stance} vote={vote} />
            ) : (
              <MissingStance key={group.uid} group={group} />
            )
          })}
        </div>
      )}

      {selected.length > 0 && vote.explanations_state === 'no_explanation' && (
        <p className="mt-3 text-[13px] text-ink-faint">
          Aucune explication de vote dans le compte rendu de la séance.
        </p>
      )}

      <p className="mt-4 border-t border-line pt-3.5 text-[13px] text-ink-faint">
        Assemblée entière : <b className="font-semibold text-yes">{vote.synthesis.votes_for}</b> pour ·{' '}
        <b className="font-semibold text-no">{vote.synthesis.votes_against}</b> contre ·{' '}
        <b className="font-semibold text-abstain">{vote.synthesis.abstentions}</b> abst.
      </p>
    </Card>
  )
}

/**
 * Sélecteur des groupes comparés : autant de places que le plafond de la
 * réponse (quatre), et une feuille qui liste tous les groupes du référentiel.
 *
 * Les places disent le maximum sans compteur. Une place pleine retire son
 * groupe d'un geste ; une place vide ouvre la feuille. La feuille montre les
 * douze groupes sans masquer ni mettre en avant aucun, dans l'ordre où la
 * réponse les donne, avec leur nom complet : l'offre reste entière, et la limite
 * est annoncée avant d'être atteinte plutôt que signalée par une erreur du
 * serveur (les groupes non choisis sont alors grisés, jamais cachés).
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
  const [open, setOpen] = useState(false)
  const full = selected.length >= max
  const places = Array.from({ length: max }, (_, index) => selected[index] ?? null)

  return (
    <>
      <ul
        aria-label={`Groupes comparés : ${selected.length} sur ${max}`}
        className="grid grid-cols-4 gap-2"
      >
        {places.map((abbrev, index) => {
          const group = abbrev === null ? null : groups.find((g) => g.abbrev === abbrev)
          return (
            // Clé = rang de la place, pas sigle : le bouton reste le même nœud
            // quand la place se remplit ou se vide, donc le focus ne se perd
            // ni à la fermeture de la feuille ni au retrait d'un groupe.
            <li key={index} className="min-w-0">
              {abbrev === null ? (
                <button
                  type="button"
                  aria-haspopup="dialog"
                  aria-label="Ajouter un groupe"
                  onClick={() => setOpen(true)}
                  className={`flex min-h-[72px] w-full flex-col items-center justify-center gap-1 rounded-[14px] border-[1.5px] border-dashed border-line-strong px-0.5 py-1 text-[13px] leading-tight font-semibold text-ink-soft transition-colors hover:bg-surface-soft ${FOCUS}`}
                >
                  <Icon name="add" className="size-[18px]" />
                  Ajouter
                </button>
              ) : (
                <button
                  type="button"
                  aria-label={`Retirer ${abbrev}${group ? ` (${group.label})` : ''}`}
                  onClick={() => onToggle(abbrev)}
                  className={`relative flex min-h-[72px] w-full flex-col items-center justify-center gap-1.5 rounded-[14px] bg-surface px-0.5 py-1 text-[13px] leading-tight font-semibold text-ink transition-colors hover:bg-surface-soft ${FOCUS}`}
                >
                  <GroupMark
                    abbrev={abbrev}
                    color={group?.color ?? null}
                    className="h-6 w-10 p-0.5"
                  />
                  <span className="max-w-full truncate">{abbrev}</span>
                  <span aria-hidden className="absolute top-1.5 right-1.5 flex text-ink-faint">
                    <Icon name="close" className="size-3.5" strokeWidth={2.4} />
                  </span>
                </button>
              )}
            </li>
          )
        })}
      </ul>

      <BottomSheet
        open={open}
        onClose={() => setOpen(false)}
        title="Groupes à comparer"
        aside={`${selected.length} sur ${max}`}
        sideOnWide
        footer={
          <SheetButton primary onClick={() => setOpen(false)}>
            Voir la comparaison
          </SheetButton>
        }
      >
        <p role="status" className="mb-1 text-[13px] leading-snug text-ink-soft">
          {full
            ? `${max} groupes sont comparés, le maximum : retirez-en un pour en changer.`
            : `Jusqu'à ${max} groupes, comparés côte à côte.`}
        </p>
        <div role="group" aria-label="Groupes">
          {groups.map((group) => {
            const on = selected.includes(group.abbrev)
            return (
              <CheckRow
                key={group.uid}
                checked={on}
                disabled={full && !on}
                onToggle={() => onToggle(group.abbrev)}
              >
                <GroupMark
                  abbrev={group.abbrev}
                  color={group.color}
                  className="h-6 w-10 p-0.5"
                />
                <span className="min-w-0 py-1 leading-tight">
                  {group.abbrev}
                  <span className="block text-xs font-normal text-ink-soft">{group.label}</span>
                </span>
              </CheckRow>
            )
          })}
        </div>
      </BottomSheet>
    </>
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

  const selectionNotes = data && hasSelection && (
    <div className="space-y-1 md:columns-2 md:gap-x-12 md:space-y-0 md:[&>p]:mb-3 md:[&>p]:break-inside-avoid">
      <p>
        {data.scope_note}{' '}
        <Link to="/scrutins" className="link">
          Voir tous les votes
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
          famille. Filtrer par sujet laisse donc de côté les textes pas
          encore rattachés.{' '}
          <Link to="/themes/methode" className="link">
            Méthode
          </Link>
        </p>
      )}
    </div>
  )

  return (
    <>
      <PageHeader
        info={selectionNotes}
        title="Comparer les votes des groupes"
        lede="Choisissez les groupes à mettre côte à côte : chaque vote sur l'ensemble d'un texte affiche leur position, en pourcentage et en voix."
      />

      <p className="mb-2">
        <Link
          to="/votes-par-groupe/methode"
          className="inline-flex min-h-11 items-center text-[15px] font-semibold link"
        >
          Comment sont retenus les votes sur l’ensemble ?
        </Link>
      </p>

      <div className="mb-4 space-y-3">
        <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between md:gap-6">
          <div className="md:w-full md:max-w-xl">
            {data ? (
              <GroupPicker
                groups={groups}
                selected={selectedAbbrevs}
                max={maxGroups}
                onToggle={toggleGroup}
              />
            ) : isLoading ? (
              <Loading>Chargement des groupes disponibles…</Loading>
            ) : null}
          </div>
          <label className="flex items-center gap-3 text-[13px] font-semibold text-ink-soft md:w-80 md:shrink-0">
            Sujet
            <span className="relative flex min-w-0 flex-1">
              <select
                value={theme}
                onChange={(e) => update({ theme: e.target.value })}
                className="h-12 min-w-0 flex-1 appearance-none rounded-xl bg-surface py-0 pr-10 pl-3.5 text-base font-semibold text-ink focus:ring-2 focus:ring-accent/40 focus:outline-none md:text-[15px]"
              >
                <option value="">Tous</option>
                {(families.data?.families ?? []).map((family) => (
                  <option key={family.code} value={family.code}>
                    {family.label}
                  </option>
                ))}
              </select>
              <Icon
                name="down"
                className="pointer-events-none absolute top-1/2 right-3.5 size-[18px] -translate-y-1/2 text-ink-faint"
              />
            </span>
          </label>
        </div>

        {groups.length > 0 && (
          /* Le renvoi vers les fiches de groupes est posé ici plutôt que dans
             la barre de navigation : c'est en choisissant les sigles qu'on se
             demande qui ils recouvrent, pas trois pages plus tôt. */
          <Link
            to="/groupes"
            className="inline-flex min-h-11 items-center text-[15px] font-semibold link"
          >
            Qui est qui : effectifs et composition des {groups.length} groupes →
          </Link>
        )}

        {/*
          Les notes de lecture sont dues (README.md §6) mais tiennent dans un
          seul encadré : quatre blocs empilés repoussaient les votes hors de
          l'écran. Le contenu est intégral, c'est le poids visuel qui baisse.
        */}
        {data && hasSelection && (
          <Note desktopOnly flat className="mt-2 border-t border-line pt-6">
            {selectionNotes}
          </Note>
        )}
      </div>

      {!hasSelection && data && (
        <div className="border-y border-line py-4 md:py-6">
          <p className="flex items-center justify-between gap-1 text-[17px] leading-snug font-[620] md:block">
            Aucun groupe n'est sélectionné au départ.
            <InfoTip label="Pourquoi aucun groupe n'est sélectionné" className="md:hidden">
              <NoSelectionHelp maxGroups={maxGroups} />
            </InfoTip>
          </p>
          <div className="mt-2 hidden md:block">
            <NoSelectionHelp maxGroups={maxGroups} />
          </div>
        </div>
      )}

      {reloading && <Loading>Chargement des votes…</Loading>}
      {isError && <ErrorPanel error={error} />}

      {data && hasSelection && !reloading && (
        <>
          <p className="mb-3 text-sm text-ink-soft md:mb-4">
            <span className="figure text-xl text-ink">
              {data.total.toLocaleString('fr-FR')}
            </span>{' '}
            vote{data.total > 1 ? 's' : ''} sur
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
            <div className="space-y-3">
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
