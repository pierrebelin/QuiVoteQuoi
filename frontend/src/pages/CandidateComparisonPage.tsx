import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { CandidateDetails } from '../components/candidates/CandidateDetails'
import { CandidateSlots } from '../components/candidates/CandidateSlots'
import { ThemeExcerpts, ThemeRow } from '../components/candidates/ThemeExcerptRows'
import { Card, ErrorPanel, Loading, Note, PageHeader, SectionTitle } from '../components/ui'
import { parseCandidateIds, sortByFamilyName } from '../lib/candidates'
import { fetchJson } from '../lib/fetchJson'
import { useMediaQuery } from '../lib/useMediaQuery'
import type { CandidateComparisonResponse } from '../types/candidates'
import type { FamiliesResponse } from '../types/themes'

const FIVE_MINUTES = 5 * 60_000
/** Cases affichées sous 768 px ; au-delà, `max_compared_candidates`. */
const NARROW_SLOTS = 2

/**
 * Comparaison factuelle : des extraits de programmes attribués, thème par
 * thème, côte à côte ; la déclaration, les sources et les groupes associés de
 * chaque candidature dessous. Aucune comparaison n'est transformée en score ou
 * verdict, et les colonnes ne sont pas alignées ligne à ligne : cela
 * suggérerait que les extraits se répondent.
 *
 * Trois requêtes, indépendantes pour qu'aucun clic ne recharge la page :
 * - la liste des candidatures, sans paramètre, jamais remontée ;
 * - les groupes associés, au gré des candidatures affichées ;
 * - les extraits, propres à la ligne de thème ouverte (voir `ThemeExcerpts`).
 * La sélection et le thème ouvert viennent de l'URL, donc sont immédiats.
 */
export default function CandidateComparisonPage() {
  const [params, setParams] = useSearchParams()
  const wide = useMediaQuery('(min-width: 768px)')
  const theme = params.get('theme') ?? ''

  const list = useQuery<CandidateComparisonResponse>({
    queryKey: ['candidats-2027', 'liste'],
    queryFn: () => fetchJson<CandidateComparisonResponse>('/api/candidats-2027'),
    staleTime: FIVE_MINUTES,
    retry: false,
  })
  const families = useQuery<FamiliesResponse>({
    queryKey: ['themes'],
    queryFn: () => fetchJson<FamiliesResponse>('/api/themes'),
    staleTime: FIVE_MINUTES,
    retry: false,
  })

  const all = useMemo(() => list.data?.candidates ?? [], [list.data])
  const sorted = useMemo(() => sortByFamilyName(all), [all])
  const max = list.data?.max_compared_candidates ?? 4
  const slotCount = wide ? max : Math.min(NARROW_SLOTS, max)

  // L'URL peut porter jusqu'à `max` noms, ou un nom que la liste ne connaît
  // plus : seuls les connus comptent. Le mobile n'en montre que les premiers.
  const requested = useMemo(() => {
    const known = new Set(all.map((candidate) => candidate.id))
    return parseCandidateIds(params.get('candidats')).filter((id) => known.has(id))
  }, [all, params])
  const slots = useMemo(() => {
    const byId = new Map(all.map((candidate) => [candidate.id, candidate]))
    return Array.from({ length: slotCount }, (_, index) => byId.get(requested[index] ?? '') ?? null)
  }, [all, requested, slotCount])
  const visible = useMemo(() => slots.flatMap((candidate) => (candidate ? [candidate] : [])), [slots])
  const visibleIds = visible.map((candidate) => candidate.id).join(',')

  // Groupes associés : ne dépendent pas du thème. L'API n'a pas de mode
  // « groupes seulement », la réponse porte aussi les extraits, ignorés ici.
  const groupsQuery = useQuery<CandidateComparisonResponse>({
    queryKey: ['candidats-2027', 'groupes', visibleIds],
    queryFn: () =>
      fetchJson<CandidateComparisonResponse>(
        `/api/candidats-2027?${new URLSearchParams({ candidats: visibleIds })}`,
      ),
    enabled: visibleIds !== '',
    placeholderData: keepPreviousData,
    staleTime: FIVE_MINUTES,
    retry: false,
  })

  function write(next: { theme?: string; candidats?: string[] }) {
    setParams(
      (previous) => {
        const updated = new URLSearchParams(previous)
        if (next.theme !== undefined) {
          if (next.theme) updated.set('theme', next.theme)
          else updated.delete('theme')
        }
        if (next.candidats !== undefined) {
          if (next.candidats.length > 0) updated.set('candidats', next.candidats.join(','))
          else updated.delete('candidats')
        }
        return updated
      },
      { replace: true },
    )
  }

  function pickCandidate(slot: number, candidateId: string) {
    const next = [...requested]
    if (slot < next.length) next[slot] = candidateId
    else next.push(candidateId)
    write({ candidats: [...new Set(next)].slice(0, max) })
  }

  function clearSlot(slot: number) {
    write({ candidats: requested.filter((_, index) => index !== slot) })
  }

  // Les groupes d'une candidature : ceux de la réponse conservée ne valent que
  // si elle la couvre, sinon la candidature attend la sienne.
  const groupsCovered = new Set(groupsQuery.data?.selected.map((candidate) => candidate.id))

  const data = list.data

  return (
    <>
      <PageHeader
        title="Candidats 2027"
        lede="Comparez des extraits de programmes déclarés par thème, puis consultez les votes des groupes parlementaires explicitement associés à chaque candidature."
      />

      {list.isLoading && <Loading>Chargement des candidatures déclarées…</Loading>}
      {list.isError && <ErrorPanel error={list.error} />}
      {data && (
        <div className="space-y-5">
          {data.declaration_note && <Note>{data.declaration_note}</Note>}

          {data.candidates.length === 0 ? (
            <Card className="px-5 py-5">
              <h3 className="text-base font-semibold">Aucune candidature déclarée n’est encore référencée</h3>
              <p className="mt-1 text-sm leading-relaxed text-ink-soft">La structure est prête : une candidature sera publiée avec sa déclaration source, son programme et les extraits rattachés aux thèmes.</p>
            </Card>
          ) : (
            <>
              <CandidateSlots
                candidates={sorted}
                slots={slots}
                onPick={pickCandidate}
                onClear={clearSlot}
              />

              <section className="space-y-2">
                <SectionTitle>Extraits par thème</SectionTitle>
                {data.proposals_note && <Note>{data.proposals_note}</Note>}
                {families.isLoading && <Loading>Chargement des thèmes…</Loading>}
                {families.isError && <ErrorPanel error={families.error} />}
                {families.data && (
                  <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card">
                    {families.data.families.map((family) => (
                      <ThemeRow
                        key={family.code}
                        family={family}
                        open={family.code === theme}
                        onToggle={() => write({ theme: family.code === theme ? '' : family.code })}
                      >
                        <ThemeExcerpts theme={family.code} candidates={visible} />
                      </ThemeRow>
                    ))}
                  </div>
                )}
              </section>

              <section className="space-y-2">
                <SectionTitle>Sur chaque candidature</SectionTitle>
                {data.groups_note && <Note>{data.groups_note}</Note>}
                {visible.length === 0 ? (
                  <p className="rounded-xl border border-line bg-surface px-4 py-4 text-sm leading-relaxed text-ink-soft shadow-card">
                    Choisissez une candidature pour lire sa déclaration, ses sources et les groupes parlementaires qui lui sont associés.
                  </p>
                ) : (
                  <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card">
                    {visible.map((candidate) => (
                      <CandidateDetails
                        key={candidate.id}
                        candidate={candidate}
                        theme={theme}
                        groupsFailed={groupsQuery.isError}
                        groups={
                          groupsCovered.has(candidate.id)
                            ? (groupsQuery.data?.parliamentary_groups.filter(
                                (group) => group.candidate_id === candidate.id,
                              ) ?? [])
                            : null
                        }
                      />
                    ))}
                  </div>
                )}
              </section>
            </>
          )}
        </div>
      )}
    </>
  )
}
