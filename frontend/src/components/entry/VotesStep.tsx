import type { ReactNode } from 'react'
import { useQueries, type UseQueryResult } from '@tanstack/react-query'
import { Link } from 'react-router'
import { ErrorPanel, Loading, Note } from '../ui'
import { fetchJson } from '../../lib/fetchJson'
import { typographic } from '../../lib/entryFlow'
import type { FinalVoteListResponse, GroupDto } from '../../types/finalVotes'
import type { FamilyDto } from '../../types/themes'
import VoteCard from '../VoteCard'
import { finalVoteCard } from '../voteCardData'
import VotesToolbar from './VotesToolbar'
import ThemeTexts from './ThemeTexts'
import { InfoTip } from '../InfoTip'

const PREVIEW_SIZE = 5

/** La page de comparaison n'accepte pas plus de quatre groupes dans son adresse. */
const MAX_GROUPS_IN_COMPARISON_LINK = 4

/**
 * Tous les votes sur l'ensemble d'un sujet. Les groupes suivis ne partent dans
 * l'adresse que si la page de destination peut les comparer ; sinon elle les
 * redemande, plutôt que d'en désigner à la place du lecteur.
 */
function allVotesLink(code: string, followed: string[]): string {
  const base = `/votes-par-groupe?theme=${encodeURIComponent(code)}`
  if (followed.length === 0 || followed.length > MAX_GROUPS_IN_COMPARISON_LINK) return base
  return `${base}&groupes=${followed.map(encodeURIComponent).join(',')}`
}

/**
 * Ce que veut dire l'issue d'un vote sur l'ensemble. Dite une fois, dans
 * l'infobulle du sujet, et non sur chaque carte.
 */
const OUTCOME_NOTE =
  '« Adoptée » veut dire que l’Assemblée a voté pour ce texte à cette lecture : ce n’est pas forcément la loi définitive, le Sénat peut encore l’examiner et le modifier. « Rejetée » est l’issue de cette lecture seulement.'

function ReadingNote() {
  return (
    <>
      Un groupe parlementaire n’est pas un parti. Les chiffres sont ceux publiés par
      l’Assemblée nationale ; le détail de chaque vote, député par député, s’ouvre depuis son
      titre. <Link to="/votes-par-groupe/methode" className="underline">Méthode</Link>
    </>
  )
}

function ThemeSection({
  family,
  query,
  groups,
  followed,
  first,
  info,
}: {
  family: FamilyDto
  query: UseQueryResult<FinalVoteListResponse>
  groups: GroupDto[]
  followed: string[]
  first: boolean
  /** Mentions du parcours : sur téléphone, portées par le titre du premier sujet. */
  info?: ReactNode
}) {
  const total = query.data?.total ?? 0

  return (
    <section aria-labelledby={`theme-${family.code}`} className="space-y-3">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h2
          id={first ? 'entry-heading' : `theme-${family.code}`}
          tabIndex={first ? -1 : undefined}
          className="text-xl font-semibold tracking-tight outline-none"
        >
          {typographic(family.label)}{' '}
          <span className="text-[15px] font-normal text-ink-soft">· dernières lois{' '}
            {/* Insécable : l'icône ne doit pas partir seule à la ligne. */}
            <span className="whitespace-nowrap">
              votées
              {info && (
                <InfoTip label="Comment lire ces votes" className="md:hidden">
                  {info}
                </InfoTip>
              )}
            </span>
          </span>
        </h2>
        {total > 0 && (
          <Link
            to={allVotesLink(family.code, followed)}
            className="inline-flex min-h-11 items-center text-sm font-medium text-accent underline"
          >
            {total > 1 ? `Voir les ${total} votes` : 'Voir le vote'}
          </Link>
        )}
      </div>

      {query.isLoading && <Loading>Chargement des votes…</Loading>}
      {query.isError && <ErrorPanel error={query.error} />}
      {query.data &&
        (query.data.items.length > 0 ? (
          <div className="space-y-3">
            {query.data.items.map((vote) => (
              <VoteCard
                key={vote.scrutin_uid}
                size="full"
                {...finalVoteCard(vote, groups, followed)}
              />
            ))}
          </div>
        ) : (
          <Note>
            Aucun vote sur l’ensemble n’est actuellement rattaché à ce sujet. Les textes non
            rattachés restent consultables ; cette absence ne permet pas de conclure qu’aucun vote
            n’a eu lieu.
          </Note>
        ))}
      <ThemeTexts family={family} />
    </section>
  )
}

/**
 * Écran 3 : les cinq votes sur l’ensemble les plus récents de chaque sujet
 * choisi, avec la position des douze groupes. Même ordre pour tous les
 * visiteurs : du plus récent au plus ancien (README.md §2).
 */
export default function VotesStep({
  families,
  themes,
  groups,
  followed,
  onToggleTheme,
  onToggleGroup,
  onAllGroups,
}: {
  families: FamilyDto[]
  /** Codes des sujets choisis, dans l'ordre de l'API. */
  themes: string[]
  /** Groupes dans l'ordre de l'hémicycle. */
  groups: GroupDto[]
  /** Sigles suivis ; vide = tous. */
  followed: string[]
  onToggleTheme: (code: string) => void
  onToggleGroup: (abbrev: string) => void
  onAllGroups: () => void
}) {
  const selected = families.filter((family) => themes.includes(family.code))

  // `tous_groupes` ramène la position de chaque groupe : la réponse ne dépend
  // donc pas des groupes suivis, qui ne servent qu'à la mise en valeur. Ne pas
  // l'associer à `groupes`, l'API répond alors 400.
  const results = useQueries({
    queries: selected.map((family) => ({
      queryKey: ['votes-finaux', 'accueil', family.code],
      queryFn: () =>
        fetchJson<FinalVoteListResponse>(
          `/api/votes-finaux?${new URLSearchParams({
            theme: family.code,
            tous_groupes: 'true',
            limit: String(PREVIEW_SIZE),
            offset: '0',
          })}`,
        ),
      retry: false,
    })),
  })

  const scopeNote = results.find((result) => result.data)?.data?.scope_note

  return (
    <div className="space-y-6">
      <VotesToolbar
        families={families}
        themes={themes}
        groups={groups}
        followed={followed}
        onToggleTheme={onToggleTheme}
        onToggleGroup={onToggleGroup}
        onAllGroups={onAllGroups}
      />

      <div className="space-y-8">
        {selected.map((family, index) => (
          <ThemeSection
            key={family.code}
            family={family}
            query={results[index]}
            groups={groups}
            followed={followed}
            first={index === 0}
            info={
              index === 0 && (
                <>
                  <p>{OUTCOME_NOTE}</p>
                  {scopeNote && <p>{scopeNote}</p>}
                  <p>
                    <ReadingNote />
                  </p>
                </>
              )
            }
          />
        ))}
      </div>

      <div className="space-y-2">
        <Note desktopOnly>{OUTCOME_NOTE}</Note>
        {scopeNote && <Note desktopOnly>{scopeNote}</Note>}
        <Note desktopOnly>
          <ReadingNote />
        </Note>
      </div>
    </div>
  )
}
