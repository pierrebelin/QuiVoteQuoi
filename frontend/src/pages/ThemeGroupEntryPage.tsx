import { useEffect, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useNavigate, useSearchParams } from 'react-router'
import GroupStep from '../components/entry/GroupStep'
import ThemeStep from '../components/entry/ThemeStep'
import VotesStep from '../components/entry/VotesStep'
import { ErrorPanel, Loading } from '../components/ui'
import {
  ENTRY_PATH,
  PARAM_GROUPS,
  PARAM_STEP,
  PARAM_THEMES,
  entrySearch,
  inHemicycleOrder,
  splitList,
  themesSummary,
  toggled,
  type EntryStep,
} from '../lib/entryFlow'
import { fetchJson } from '../lib/fetchJson'
import type { FinalVoteListResponse } from '../types/finalVotes'
import type { FamiliesResponse } from '../types/themes'

/**
 * Votes par sujet : trois écrans, une seule adresse. L'état du parcours vit
 * dans l'URL, pour qu'un lien partagé rouvre exactement les mêmes votes.
 *
 *   /par-sujet                                  → 1. sujets
 *   /par-sujet?sujets=a,b&etape=groupes         → 2. groupes à suivre
 *   /par-sujet?sujets=a,b                       → 3. votes, tous les groupes
 *   /par-sujet?sujets=a,b&suivis=GDR,RN         → 3. votes, GDR et RN mis en valeur
 *
 * `etape=sujets` rouvre l'écran 1 avec les sujets déjà cochés (retour depuis
 * l'écran 2). Sans `suivis`, tous les groupes sont suivis : aucun groupe n'est
 * présélectionné (README.md §6).
 */
export default function ThemeGroupEntryPage() {
  const [params] = useSearchParams()
  const navigate = useNavigate()
  const pendingFocus = useRef(false)

  const requestedThemes = splitList(params.get(PARAM_THEMES))
  const requestedFollowed = splitList(params.get(PARAM_GROUPS))
  const requestedStep = params.get(PARAM_STEP)

  const families = useQuery<FamiliesResponse>({
    queryKey: ['themes'],
    queryFn: () => fetchJson<FamiliesResponse>('/api/themes'),
    retry: false,
  })

  const allFamilies = families.data?.families ?? []
  // Dans l'ordre de l'API, quel que soit l'ordre de l'adresse, et sans code inconnu.
  const themes = allFamilies
    .filter((family) => requestedThemes.includes(family.code))
    .map((family) => family.code)
  const hasThemes = families.data ? themes.length > 0 : requestedThemes.length > 0
  const step: EntryStep =
    !hasThemes || requestedStep === 'sujets'
      ? 'themes'
      : requestedStep === 'groupes'
        ? 'groups'
        : 'votes'

  // Le référentiel des groupes voyage avec la liste des votes ; une ligne
  // suffit, on n'en lit que `groups`.
  const groupsQuery = useQuery<FinalVoteListResponse, Error, FinalVoteListResponse['groups']>({
    queryKey: ['votes-finaux', 'groupes'],
    queryFn: () => fetchJson<FinalVoteListResponse>('/api/votes-finaux?limit=1&offset=0'),
    select: (data) => inHemicycleOrder(data.groups),
    enabled: step !== 'themes',
    retry: false,
  })

  const groups = groupsQuery.data ?? []
  const followedKnown = groups
    .filter((group) => requestedFollowed.includes(group.abbrev))
    .map((group) => group.abbrev)
  // Tous les groupes cochés un à un, c'est « tous les groupes » : une seule forme.
  const followed = followedKnown.length === groups.length ? [] : followedKnown

  const ready = families.data !== undefined && (step === 'themes' || groups.length > 0)

  // Après un changement d'écran, remonter en haut et poser le focus sur le titre.
  useEffect(() => {
    if (!pendingFocus.current || !ready) return
    pendingFocus.current = false
    window.scrollTo({ top: 0 })
    document.getElementById('entry-heading')?.focus({ preventScroll: true })
  }, [step, ready])

  function go(
    next: { themes?: string[]; followed?: string[]; step?: EntryStep },
    mode: 'push' | 'replace' = 'replace',
  ) {
    if (mode === 'push') pendingFocus.current = true
    navigate(
      {
        pathname: ENTRY_PATH,
        search: entrySearch({
          themes: next.themes ?? themes,
          followed: next.followed ?? followed,
          step: next.step ?? step,
        }),
      },
      { replace: mode === 'replace' },
    )
  }

  function toggleTheme(code: string) {
    const next = toggled(themes, code)
    go({ themes: allFamilies.filter((f) => next.includes(f.code)).map((f) => f.code) })
  }

  function toggleGroup(abbrev: string) {
    const next = toggled(followed, abbrev)
    const ordered = groups.filter((g) => next.includes(g.abbrev)).map((g) => g.abbrev)
    go({ followed: ordered.length === groups.length ? [] : ordered })
  }

  if (families.isError) return <ErrorPanel error={families.error} />
  if (!families.data) return <Loading>Chargement des sujets…</Loading>

  if (step === 'themes') {
    return (
      <ThemeStep
        families={allFamilies}
        selected={themes}
        onToggle={toggleTheme}
        onNext={() => go({ step: 'groups' }, 'push')}
      />
    )
  }

  if (groupsQuery.isError) return <ErrorPanel error={groupsQuery.error} />
  if (groups.length === 0) return <Loading>Chargement des groupes…</Loading>

  if (step === 'groups') {
    return (
      <GroupStep
        groups={groups}
        followed={followed}
        themesLabel={themesSummary(
          allFamilies.filter((f) => themes.includes(f.code)).map((f) => f.label),
        )}
        backTo={`${ENTRY_PATH}${entrySearch({ themes, followed, step: 'themes' })}`}
        onToggle={toggleGroup}
        onAll={() => go({ followed: [] })}
        onNext={() => go({ step: 'votes' }, 'push')}
      />
    )
  }

  return (
    <VotesStep
      families={allFamilies}
      themes={themes}
      groups={groups}
      followed={followed}
      onToggleTheme={toggleTheme}
      onToggleGroup={toggleGroup}
      onAllGroups={() => go({ followed: [] })}
    />
  )
}
