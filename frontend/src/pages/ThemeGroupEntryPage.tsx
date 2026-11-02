import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { FamilyIllustration } from '../components/FamilyIllustration'
import { PoliticalLogo } from '../components/PoliticalLogo'
import { GROUP_LOGOS } from '../content/logos'
import { Card, ErrorPanel, Loading, Note, Pill } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import type { FamiliesResponse, FamilyDto } from '../types/themes'
import type {
  FinalVoteDto,
  FinalVoteListResponse,
  GroupDto,
  StanceDto,
} from '../types/finalVotes'
import { formatDate } from '../types/scrutins'

const PREVIEW_SIZE = 5
const FIRST_THEMES_COUNT = 8
const MAX_GROUPS_BEFORE_LOAD = 4

function themeButtonClass(selected: boolean) {
  return `group flex items-center gap-3 rounded-xl border p-2 pr-3 text-left text-sm font-medium transition-colors ${
    selected
      ? 'border-ink bg-ink text-white'
      : 'border-line bg-surface text-ink-soft shadow-card hover:border-line-strong hover:text-ink'
  }`
}

/** Le selecteur n'emploie pas de sigle seul : le premier geste doit rester
 * comprehensible sans connaitre l'organisation de l'Assemblee. */
function GroupChoice({
  group,
  selected,
  disabled,
  onClick,
}: {
  group: GroupDto
  selected: boolean
  disabled: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      disabled={disabled}
      onClick={onClick}
      style={{ borderLeftColor: group.color ?? undefined }}
      className={`flex items-center gap-3 rounded-xl border border-l-4 p-2 pr-3 text-left transition-colors ${
        selected
          ? 'border-ink bg-ink text-white'
          : 'border-line bg-surface text-ink-soft shadow-card hover:border-line-strong hover:text-ink disabled:cursor-not-allowed disabled:opacity-45'
      }`}
    >
      <PoliticalLogo logo={GROUP_LOGOS[group.abbrev]} fallback={group.abbrev} color={group.color} />
      <span className="min-w-0">
        <span className="block text-sm font-semibold">{group.label}</span>
        <span className={`mt-0.5 block text-xs ${selected ? 'text-white/70' : 'text-ink-faint'}`}>
          {group.abbrev} · {group.final_vote_count} vote{group.final_vote_count > 1 ? 's' : ''} sur l’ensemble publié{group.final_vote_count > 1 ? 's' : ''}
        </span>
      </span>
    </button>
  )
}

function GroupTally({ stance, group }: { stance?: StanceDto; group: GroupDto }) {
  if (!stance) {
    return (
      <p className="mt-1 text-sm leading-relaxed text-ink-faint">
        Aucune ligne n’est publiée pour ce groupe sur ce vote. Cette absence ne vaut ni abstention ni position.
      </p>
    )
  }

  const nonVoters = stance.tally.not_voting + stance.tally.voluntary_not_voting
  return (
    <div className="mt-1 text-sm leading-relaxed text-ink-soft">
      <p className="font-medium text-ink">{group.label} ({group.abbrev})</p>
      <p className="mt-0.5">
        <span className="font-semibold text-yes">{stance.tally.votes_for} pour</span> ·{' '}
        <span className="font-semibold text-no">{stance.tally.votes_against} contre</span> ·{' '}
        <span className="font-semibold text-abstain">{stance.tally.abstentions} abstentions</span>
        {nonVoters > 0 && <> · {nonVoters} non-votant{nonVoters > 1 ? 's' : ''}</>}
      </p>
    </div>
  )
}

/** Une carte est volontairement la meme quel que soit le groupe choisi : le
 * groupe change la ventilation, jamais la selection ni l'ordre des actes. */
function FinalVoteCard({ vote, selected }: { vote: FinalVoteDto; selected: GroupDto[] }) {
  return (
    <Card className="px-4 py-4 sm:px-5">
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <Pill tone={vote.adopted ? 'yes' : 'no'}>{vote.outcome_label}</Pill>
        <span className="text-xs text-ink-faint">{formatDate(vote.date)} · vote sur l’ensemble</span>
      </div>

      <h3 className="mt-2 text-base font-semibold leading-snug">
        <Link to={`/textes/${encodeURIComponent(vote.text_key)}`} className="hover:text-accent">
          {vote.text_label}
        </Link>
      </h3>

      <section className="mt-4 border-t border-line pt-3">
        <h4 className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">Ce que prévoit le texte</h4>
        {vote.official_text ? (
          <p className="mt-1 text-sm leading-relaxed text-ink-soft">
            Version identifiée : {vote.official_text.version_label}
            {vote.official_text.document_published_on && `, publiée le ${formatDate(vote.official_text.document_published_on)}`}.
            {' '}La synthèse de cette version n’est pas encore publiée ; le document officiel reste accessible ci-dessous.
          </p>
        ) : (
          <p className="mt-1 text-sm leading-relaxed text-ink-faint">
            La version exacte examinée lors de ce vote n’est pas encore référencée. Aucune synthèse de contenu n’est donc affichée.
          </p>
        )}
      </section>

      <section className="mt-4 border-t border-line pt-3">
        <h4 className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">Vote de l’Assemblée</h4>
        <p className="mt-1 text-sm leading-relaxed text-ink-soft">
          {vote.outcome_label} · {vote.synthesis.votes_for} pour · {vote.synthesis.votes_against} contre · {vote.synthesis.abstentions} abstentions.
        </p>
      </section>

      {selected.length > 0 && (
        <section className="mt-4 border-t border-line pt-3">
          <h4 className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
            {selected.length === 1 ? 'Groupe sélectionné' : 'Groupes sélectionnés'}
          </h4>
          <div className="mt-2 grid gap-3 lg:grid-cols-2">
            {selected.map((group) => (
              <GroupTally
                key={group.uid}
                group={group}
                stance={vote.stances.find((stance) => stance.group_uid === group.uid)}
              />
            ))}
          </div>
        </section>
      )}

      <div className="mt-4 flex flex-wrap gap-x-4 gap-y-2 text-sm">
        {vote.official_text && (
          <a href={vote.official_text.official_url} target="_blank" rel="noreferrer" className="text-accent underline">
            Lire le texte intégral
          </a>
        )}
        <Link to={`/scrutins/${vote.scrutin_uid}`} className="text-accent underline">
          Voir le détail du vote
        </Link>
        {vote.official_text && (
          <a href={vote.official_text.mapping_source_url} target="_blank" rel="noreferrer" className="text-accent underline">
            Source officielle du rattachement
          </a>
        )}
      </div>
    </Card>
  )
}

function familyLabel(families: FamilyDto[], code: string) {
  return families.find((family) => family.code === code)?.label ?? code
}

/**
 * Entrée lecteur du site : choix fermé d'un thème, puis d'un groupe, avant
 * toute terminologie institutionnelle. Les paramètres rendent le parcours
 * partageable sans personnaliser l’aperçu.
 */
export default function ThemeGroupEntryPage() {
  const [params, setParams] = useSearchParams()
  const [showAllThemes, setShowAllThemes] = useState(false)
  const [showComparison, setShowComparison] = useState(false)
  const theme = params.get('theme') ?? ''
  const requested = (params.get('groupes') ?? '')
    .split(',')
    .map((group) => group.trim())
    .filter(Boolean)

  const families = useQuery<FamiliesResponse>({
    queryKey: ['themes'],
    queryFn: () => fetchJson<FamiliesResponse>('/api/themes'),
    retry: false,
  })

  const finalVotes = useQuery<FinalVoteListResponse>({
    queryKey: ['votes-finaux', 'entree', theme, requested.join(',')],
    enabled: Boolean(theme),
    queryFn: () => {
      const query = new URLSearchParams({
        theme,
        groupes: requested.join(','),
        limit: String(requested.length > 0 ? PREVIEW_SIZE : 1),
        offset: '0',
      })
      return fetchJson<FinalVoteListResponse>(`/api/votes-finaux?${query}`)
    },
    retry: false,
  })

  function update(next: { theme?: string; groups?: string[] }) {
    const updated = new URLSearchParams()
    const nextTheme = next.theme ?? theme
    const nextGroups = next.groups ?? requested
    if (nextTheme) updated.set('theme', nextTheme)
    if (nextGroups.length > 0) updated.set('groupes', nextGroups.join(','))
    setParams(updated)
  }

  function chooseTheme(code: string) {
    setShowComparison(false)
    update({ theme: code, groups: [] })
  }

  const groups = finalVotes.data?.groups ?? []
  const selected = finalVotes.data?.selected ?? []
  const maxGroups = finalVotes.data?.max_compared_groups ?? MAX_GROUPS_BEFORE_LOAD
  const hasGroup = requested.length > 0
  const visibleThemes = showAllThemes
    ? families.data?.families ?? []
    : (families.data?.families ?? []).slice(0, FIRST_THEMES_COUNT)
  const showAllGroupChoices = !hasGroup || showComparison || requested.length > 1
  const selectedTokens = finalVotes.data ? selected.map((group) => group.abbrev) : requested

  function toggleGroup(abbrev: string) {
    const next = selectedTokens.includes(abbrev)
      ? selectedTokens.filter((token) => token !== abbrev)
      : [...selectedTokens, abbrev].slice(0, maxGroups)
    update({ groups: next })
  }

  return (
    <div className="space-y-8">
      <section className="max-w-4xl">
        <p className="text-xs font-semibold uppercase tracking-[0.08em] text-accent">Votes de l’Assemblée nationale</p>
        <h2 className="mt-2 text-3xl font-semibold tracking-tight sm:text-4xl">
          Comment ont voté les groupes sur…
        </h2>
        <p className="mt-3 max-w-3xl text-base leading-relaxed text-ink-soft">
          Choisissez un thème. Regardez ce que les groupes parlementaires ont voté, sur des textes et des sources accessibles.
        </p>
      </section>

      <section aria-labelledby="themes-heading">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h3 id="themes-heading" className="text-sm font-semibold">1. Choisissez un thème</h3>
          <Link to="/themes/methode" className="text-xs text-accent underline">Méthode de rattachement</Link>
        </div>
        {families.isLoading && <Loading>Chargement des thèmes…</Loading>}
        {families.isError && <ErrorPanel error={families.error} />}
        {families.data && (
          <>
            <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
              {visibleThemes.map((family) => (
                <button
                  key={family.code}
                  type="button"
                  aria-pressed={theme === family.code}
                  onClick={() => chooseTheme(family.code)}
                  className={themeButtonClass(theme === family.code)}
                >
                  <FamilyIllustration code={family.code} className="size-10 shrink-0 rounded-lg" glyphClassName="size-6" />
                  <span>{family.label}</span>
                </button>
              ))}
            </div>
            {!showAllThemes && families.data.families.length > FIRST_THEMES_COUNT && (
              <button
                type="button"
                onClick={() => setShowAllThemes(true)}
                className="mt-3 text-sm font-medium text-accent underline"
              >
                Voir les {families.data.families.length} thèmes
              </button>
            )}
          </>
        )}
      </section>

      {theme && (
        <section aria-labelledby="groups-heading" className="border-t border-line pt-6">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <div>
              <h3 id="groups-heading" className="text-sm font-semibold">2. Quel groupe voulez-vous regarder ?</h3>
              <p className="mt-1 text-sm text-ink-soft">
                {familyLabel(families.data?.families ?? [], theme)} · un groupe parlementaire n’est pas nécessairement un parti.
              </p>
            </div>
            <Link to="/groupes" className="text-xs text-accent underline">Composition des groupes</Link>
          </div>

          {finalVotes.isLoading && <Loading>Chargement des groupes…</Loading>}
          {finalVotes.isError && <ErrorPanel error={finalVotes.error} />}
          {finalVotes.data && (
            <>
              {showAllGroupChoices ? (
                <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                  {groups.map((group) => (
                    <GroupChoice
                      key={group.uid}
                      group={group}
                      selected={selectedTokens.includes(group.abbrev)}
                      disabled={selectedTokens.length >= maxGroups && !selectedTokens.includes(group.abbrev)}
                      onClick={() => toggleGroup(group.abbrev)}
                    />
                  ))}
                </div>
              ) : (
                <div className="mt-3 flex flex-wrap items-center gap-3">
                  {selected.map((group) => <GroupChoice key={group.uid} group={group} selected disabled={false} onClick={() => toggleGroup(group.abbrev)} />)}
                  <button type="button" onClick={() => setShowComparison(true)} className="text-sm font-medium text-accent underline">
                    Ajouter un groupe pour comparer
                  </button>
                </div>
              )}
              {selectedTokens.length >= maxGroups && (
                <p className="mt-3 text-xs text-ink-faint">La comparaison est limitée à {maxGroups} groupes pour conserver les chiffres bruts lisibles.</p>
              )}
            </>
          )}
        </section>
      )}

      {hasGroup && finalVotes.data && (
        <section aria-labelledby="votes-heading" className="border-t border-line pt-6">
          <div className="flex flex-wrap items-baseline justify-between gap-x-5 gap-y-2">
            <div>
              <h3 id="votes-heading" className="text-xl font-semibold tracking-tight">Les cinq actes les plus récents</h3>
              <p className="mt-1 text-sm text-ink-soft">
                Même ordre pour tous les visiteurs : du plus récent au plus ancien. Le résultat vaut pour cette lecture, pas pour le devenir final de la loi.
              </p>
            </div>
            <Link to={`/votes-par-groupe?theme=${encodeURIComponent(theme)}&groupes=${encodeURIComponent(selectedTokens.join(','))}`} className="text-sm font-medium text-accent underline">
              Voir tous les votes finaux du thème
            </Link>
          </div>

          {finalVotes.data.items.length > 0 ? (
            <div className="mt-4 space-y-4">
              {finalVotes.data.items.map((vote) => <FinalVoteCard key={vote.scrutin_uid} vote={vote} selected={selected} />)}
            </div>
          ) : (
            <Note>
              Aucun vote sur l’ensemble n’est actuellement rattaché à ce thème. Les textes non rattachés restent consultables ; cette absence ne permet pas de conclure qu’aucun vote n’a eu lieu.
            </Note>
          )}
        </section>
      )}
    </div>
  )
}
