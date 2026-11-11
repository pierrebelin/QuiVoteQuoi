import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { useMemo, useState } from 'react'
import { useSearchParams } from 'react-router'
import { CandidateDetails } from '../components/candidates/CandidateDetails'
import { CandidateSlots } from '../components/candidates/CandidateSlots'
import { SubjectCandidates, type FaceToFace } from '../components/candidates/SubjectCandidates'
import { FaceToFaceBox, SubjectButton, SubjectList } from '../components/candidates/SubjectPicker'
import { ThemeExcerpts, ThemeRow } from '../components/candidates/ThemeExcerptRows'
import { Card, ErrorPanel, Loading, Note, PageHeader, SectionTitle } from '../components/ui'
import { parseCandidateIds, shuffleWithSeed, sortByFamilyName } from '../lib/candidates'
import { fetchJson } from '../lib/fetchJson'
import { useMediaQuery } from '../lib/useMediaQuery'
import type { CandidateComparisonResponse } from '../types/candidates'
import type { FamiliesResponse } from '../types/themes'

const FIVE_MINUTES = 5 * 60_000
/** Cases affichées sous 768 px ; au-delà, `max_compared_candidates`. */
const NARROW_SLOTS = 2
/** Valeur du paramètre `vue` de l'onglet 2 ; sans paramètre, l'onglet Par sujet. */
const FACE_TO_FACE_VIEW = 'face-a-face'
const FOCUS =
  'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

type View = 'sujet' | 'face'

/**
 * Deux lectures, en onglets :
 * - Par sujet : un sujet, toutes les candidatures qui ont un extrait dessus,
 *   dans un ordre tiré au hasard à l'arrivée sur la page (aucune n'est toujours
 *   en tête), les autres regroupées en bas ;
 * - Face-à-face : jusqu'à 4 candidatures côte à côte (2 sous 768 px), thème
 *   par thème, avec leur déclaration, leurs sources et leurs groupes associés.
 * Aucune comparaison n'est transformée en score ou verdict, et les colonnes du
 * face-à-face ne sont pas alignées ligne à ligne : cela suggérerait que les
 * extraits se répondent.
 *
 * L'onglet (`vue`), le sujet (`theme`) et le face-à-face (`candidats`) vivent
 * dans l'URL : le lien se partage, et chaque clic est immédiat.
 */
export default function CandidateComparisonPage() {
  const [params, setParams] = useSearchParams()
  const wide = useMediaQuery('(min-width: 768px)')
  const theme = params.get('theme') ?? ''
  const view: View = params.get('vue') === FACE_TO_FACE_VIEW ? 'face' : 'sujet'
  // Tirée une fois par visite : l'ordre ne bouge plus d'un sujet à l'autre.
  const [seed] = useState(() => Math.floor(Math.random() * 2 ** 32))

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
  const shuffled = useMemo(() => shuffleWithSeed(all, seed), [all, seed])
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

  // Sujet de l'onglet Par sujet : celui de l'URL s'il existe, sinon le premier.
  // Le face-à-face, lui, peut n'avoir aucun thème ouvert.
  const familyList = families.data?.families ?? []
  const subject = familyList.find((family) => family.code === theme) ?? familyList[0]

  // Groupes associés : ne dépendent pas du thème. L'API n'a pas de mode
  // « groupes seulement », la réponse porte aussi les extraits, ignorés ici.
  const groupsQuery = useQuery<CandidateComparisonResponse>({
    queryKey: ['candidats-2027', 'groupes', visibleIds],
    queryFn: () =>
      fetchJson<CandidateComparisonResponse>(
        `/api/candidats-2027?${new URLSearchParams({ candidats: visibleIds })}`,
      ),
    enabled: view === 'face' && visibleIds !== '',
    placeholderData: keepPreviousData,
    staleTime: FIVE_MINUTES,
    retry: false,
  })

  function write(next: { theme?: string; candidats?: string[]; vue?: View }) {
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
        if (next.vue !== undefined) {
          if (next.vue === 'face') updated.set('vue', FACE_TO_FACE_VIEW)
          else updated.delete('vue')
        }
        return updated
      },
      // Changer d'onglet s'inscrit dans l'historique, le reste le remplace.
      { replace: next.vue === undefined },
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

  function removeFromFaceToFace(candidateId: string) {
    write({ candidats: requested.filter((id) => id !== candidateId) })
  }

  const faceToFace: FaceToFace = {
    ids: new Set(visibleIds ? visibleIds.split(',') : []),
    full: visible.length >= slotCount,
    max: slotCount,
    toggle: (candidateId) => {
      if (visible.some((candidate) => candidate.id === candidateId)) removeFromFaceToFace(candidateId)
      else if (visible.length < slotCount) {
        // Les cases cachées sous 768 px cèdent leur place : la nouvelle doit se voir.
        write({ candidats: [...visible.map((candidate) => candidate.id), candidateId] })
      }
    },
  }

  // Les groupes d'une candidature : ceux de la réponse conservée ne valent que
  // si elle la couvre, sinon la candidature attend la sienne.
  const groupsCovered = new Set(groupsQuery.data?.selected.map((candidate) => candidate.id))

  const data = list.data

  return (
    <>
      <PageHeader
        info={
          data && (
            <>
              {data.declaration_note && <p>{data.declaration_note}</p>}
              {view === 'sujet' && data.proposals_note && <p>{data.proposals_note}</p>}
            </>
          )
        }
        title="Candidats 2027"
        lede="Comparez des extraits de programmes déclarés par sujet, puis consultez les votes des groupes parlementaires explicitement associés à chaque candidature."
      />

      {list.isLoading && <Loading>Chargement des candidatures déclarées…</Loading>}
      {list.isError && <ErrorPanel error={list.error} />}
      {data && (
        <div className="space-y-5">
          {data.declaration_note && (
            <Note desktopOnly>
              {data.declaration_note}
            </Note>
          )}

          {data.candidates.length === 0 ? (
            <Card className="px-5 py-5">
              <h3 className="text-base font-semibold">Aucune candidature déclarée n’est encore référencée</h3>
              <p className="mt-1 text-sm leading-relaxed text-ink-soft">La structure est prête : une candidature sera publiée avec sa déclaration source, son programme et les extraits rattachés aux sujets.</p>
            </Card>
          ) : (
            <>
              <ViewTabs view={view} faceCount={visible.length} onChange={(vue) => write({ vue })} />

              {families.isLoading && <Loading>Chargement des sujets…</Loading>}
              {families.isError && <ErrorPanel error={families.error} />}

              {view === 'sujet' && subject && (
                <div className="md:grid md:grid-cols-[minmax(0,240px)_minmax(0,1fr)] md:items-start md:gap-6 lg:grid-cols-[280px_minmax(0,1fr)]">
                  <aside className="hidden md:block">
                    <SubjectList
                      families={familyList}
                      current={subject.code}
                      onPick={(code) => write({ theme: code })}
                    />
                    <FaceToFaceBox
                      selected={visible}
                      max={slotCount}
                      onRemove={removeFromFaceToFace}
                      onCompare={() => write({ vue: 'face' })}
                    />
                  </aside>
                  <section className="min-w-0">
                    <div className="md:hidden">
                      <SubjectButton
                        families={familyList}
                        current={subject}
                        onPick={(code) => write({ theme: code })}
                      />
                    </div>
                    <div className="hidden md:block">
                      <SectionTitle>{`Extraits · ${subject.label}`}</SectionTitle>
                      {data.proposals_note && (
                        <div className="mb-3">
                          <Note>{data.proposals_note}</Note>
                        </div>
                      )}
                    </div>
                    <SubjectCandidates
                      theme={subject.code}
                      candidates={shuffled}
                      faceToFace={faceToFace}
                      wide={wide}
                    />
                  </section>
                </div>
              )}

              {view === 'face' && (
                <>
                  <CandidateSlots
                    candidates={sorted}
                    slots={slots}
                    onPick={pickCandidate}
                    onClear={clearSlot}
                  />

                  <section className="space-y-2">
                    <SectionTitle info={data.proposals_note}>Extraits par sujet</SectionTitle>
                    {data.proposals_note && (
                      <Note desktopOnly>
                        {data.proposals_note}
                      </Note>
                    )}
                    {families.data && (
                      <div className="divide-y divide-line overflow-hidden rounded border border-line bg-surface shadow-card">
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
                    <SectionTitle info={data.groups_note}>Sur chaque candidature</SectionTitle>
                    {data.groups_note && (
                      <Note desktopOnly>
                        {data.groups_note}
                      </Note>
                    )}
                    {visible.length === 0 ? (
                      <p className="rounded border border-line bg-surface px-4 py-4 text-sm leading-relaxed text-ink-soft shadow-card">
                        Choisissez une candidature pour lire sa déclaration, ses sources et les groupes parlementaires qui lui sont associés.
                      </p>
                    ) : (
                      <div className="divide-y divide-line overflow-hidden rounded border border-line bg-surface shadow-card">
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
            </>
          )}
        </div>
      )}
    </>
  )
}

/** Onglets Par sujet / Face-à-face ; le second montre combien de cases sont prises. */
function ViewTabs({
  view,
  faceCount,
  onChange,
}: {
  view: View
  faceCount: number
  onChange: (view: View) => void
}) {
  const options: { value: View; label: string }[] = [
    { value: 'sujet', label: 'Par sujet' },
    { value: 'face', label: 'Face-à-face' },
  ]
  return (
    <div role="group" aria-label="Vue" className="grid w-full grid-cols-2 gap-1 rounded bg-surface-soft p-[3px] md:w-80">
      {options.map((option) => {
        const on = view === option.value
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={on}
            onClick={() => onChange(option.value)}
            className={`flex min-h-9 items-center justify-center gap-1.5 rounded px-2 text-[13.5px] font-semibold whitespace-nowrap transition-colors md:text-sm ${FOCUS} ${
              on ? 'bg-surface text-ink shadow-card' : 'text-ink-soft hover:text-ink'
            }`}
          >
            {option.label}
            {option.value === 'face' && faceCount > 0 && (
              <span className="inline-block min-w-[18px] rounded-full bg-accent px-[5px] text-[11px] leading-[18px] text-surface tabular-nums">
                <span className="sr-only">, candidatures choisies : </span>
                {faceCount}
              </span>
            )}
          </button>
        )
      })}
    </div>
  )
}
