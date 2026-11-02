import { useState } from 'react'
import { useParams, Link, useSearchParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import DossierAmendments from '../components/DossierAmendments'
import DossierScrutins from '../components/DossierScrutins'
import DossierGroupActions from '../components/DossierGroupActions'
import DossierFinalScrutin from '../components/DossierFinalScrutin'
import { OutcomePanel } from '../components/OutcomeBadge'
import { Card, ErrorPanel, Loading, Pill, SectionTitle } from '../components/ui'
import type { InitiatorDto, OutcomeDto } from '../types/dossiers'

/** Actes visibles avant dépliage. Un dossier peut en porter des dizaines. */
const VISIBLE_ACTS = 3

interface ActeDto {
  date: string
  label: string
}

interface StageDto {
  label: string
  chamber: string
}

interface DocumentDto {
  document_uid: string
  title: string
  short_title: string | null
  doc_type: string
  date: string | null
  official_url: string | null
  source_archive_url: string | null
  source_license: string | null
  source_retrieved_at: string | null
}

interface DossierDetailDto {
  uid: string
  title: string
  procedure: string
  last_activity_date: string
  last_activity_label: string
  acts: ActeDto[]
  documents: DocumentDto[]
  current_stage: StageDto | null
  initiators: InitiatorDto[]
  committee: string | null
  outcome: OutcomeDto
}

/**
 * Fiche d'un dossier, en lecture seule. Sauvegarde et curation passent
 * désormais par l'API, jamais par cet écran : le navigateur d'un visiteur n'a
 * rien à écrire, et le jeton du jour n'a donc plus à y être collé.
 */
export default function DossierDetailPage() {
  const { uid } = useParams<{ uid: string }>()
  const [searchParams, setSearchParams] = useSearchParams()
  const [allActsShown, setAllActsShown] = useState(false)

  const { data, isLoading, isError, error } = useQuery<DossierDetailDto>({
    queryKey: ['dossier', uid],
    queryFn: () =>
      fetch(`/api/dossiers/${uid}`).then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return res.json()
      }),
    enabled: !!uid,
  })

  if (isLoading) return <Loading>Chargement du dossier…</Loading>

  if (isError) {
    return (
      <div>
        <Link
          to="/"
          className="mb-4 inline-block text-sm font-medium text-accent hover:underline"
        >
          ← Retour à la liste
        </Link>
        <ErrorPanel error={error} />
      </div>
    )
  }

  if (!data) return null

  const activeTab = searchParams.get('tab') === 'tableaux' ? 'tableaux' : 'actes'
  const selectTab = (tab: 'actes' | 'tableaux') => {
    const next = new URLSearchParams(searchParams)
    if (tab === 'actes') next.delete('tab')
    else next.set('tab', tab)
    setSearchParams(next)
  }

  return (
    <div>
      <Link
        to="/"
        className="mb-4 inline-block text-sm font-medium text-accent hover:underline"
      >
        ← Retour à la liste
      </Link>

      <div className="mb-5 min-w-0">
        <h2 className="max-w-4xl text-2xl font-semibold leading-tight tracking-tight">
          {data.title}
        </h2>
        <div className="mt-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-ink-faint">
          <span>{data.procedure}</span>
          {data.current_stage && (
            <Pill tone="info">
              {data.current_stage.label}
              {data.current_stage.chamber && ` — ${data.current_stage.chamber}`}
            </Pill>
          )}
          <span>·</span>
          <span className="text-ink-soft">{data.last_activity_label}</span>
          <span>·</span>
          <span>
            {new Date(data.last_activity_date + 'T00:00:00').toLocaleDateString(
              'fr-FR',
              { day: 'numeric', month: 'long', year: 'numeric' },
            )}
          </span>
          {data.committee && (
            <>
              <span>·</span>
              <span>Commission : {data.committee}</span>
            </>
          )}
        </div>
      </div>

      <div className="mb-4">
        <OutcomePanel
          outcome={data.outcome}
          lastActivityLabel={data.last_activity_label}
          lastActivityDate={data.last_activity_date}
        />
      </div>

      {/* Le groupe d'un initiateur n'est jamais affiché sans la date à laquelle
          il a été lu : l'appartenance est datée (README.md §3). */}
      {data.initiators.length > 0 && (
        <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1 mb-4 text-xs">
          <span className="text-ink-faint">
            Initiateur{data.initiators.length > 1 ? 's' : ''} :
          </span>
          {data.initiators.map((init, i) => (
            <span
              key={i}
              className="inline-flex items-center gap-1 rounded-md bg-surface px-1.5 py-0.5 text-ink-soft ring-1 ring-inset ring-line"
            >
              {init.official_url ? (
                <a
                  href={init.official_url}
                  target="_blank"
                  rel="noreferrer"
                  className="hover:text-accent hover:underline"
                >
                  {init.full_name}
                </a>
              ) : (
                init.full_name
              )}
              {init.role && <span className="text-ink-faint">{init.role}</span>}
              {init.group && init.reference_date && (
                <span className="font-medium text-ink" title={init.group.label}>
                  {init.group.abbrev}
                  <span className="ml-1 font-normal text-ink-faint">
                    (au{' '}
                    {new Date(
                      init.reference_date + 'T00:00:00',
                    ).toLocaleDateString('fr-FR', {
                      day: 'numeric',
                      month: 'short',
                      year: 'numeric',
                    })}
                    )
                  </span>
                </span>
              )}
            </span>
          ))}
        </div>
      )}

      <DossierFinalScrutin uid={data.uid} />

      <div className="mb-6 grid gap-4 md:grid-cols-2">
        <Card className="p-4">
          <SectionTitle>Repères du dossier</SectionTitle>
          <dl className="mt-3 grid gap-x-5 gap-y-3 text-sm sm:grid-cols-2">
            <div>
              <dt className="text-xs text-ink-faint">Procédure</dt>
              <dd className="mt-0.5 font-medium text-ink">{data.procedure}</dd>
            </div>
            <div>
              <dt className="text-xs text-ink-faint">Dernière activité</dt>
              <dd className="mt-0.5 font-medium text-ink">
                {new Date(data.last_activity_date + 'T00:00:00').toLocaleDateString(
                  'fr-FR',
                  { day: 'numeric', month: 'long', year: 'numeric' },
                )}
              </dd>
              <dd className="text-xs text-ink-soft">{data.last_activity_label}</dd>
            </div>
            {data.current_stage && (
              <div>
                <dt className="text-xs text-ink-faint">Étape publiée</dt>
                <dd className="mt-0.5 font-medium text-ink">
                  {data.current_stage.label}
                  {data.current_stage.chamber && ` — ${data.current_stage.chamber}`}
                </dd>
              </div>
            )}
            <div>
              <dt className="text-xs text-ink-faint">Actes enregistrés</dt>
              <dd className="mt-0.5 font-medium text-ink">
                {data.acts.length} acte{data.acts.length > 1 ? 's' : ''}
              </dd>
            </div>
          </dl>
        </Card>

        <Card className="p-4">
          <SectionTitle>Déroulé</SectionTitle>
          {data.acts.length === 0 ? (
            <p className="text-ink-faint text-sm">Aucun acte enregistré</p>
          ) : (
            (() => {
              // La source les rend du plus ancien au plus récent. On montre les
              // derniers actes en premier: c'est l'état courant du dossier.
              const recentFirst = [...data.acts].reverse()
              const hidden = recentFirst.length - VISIBLE_ACTS
              const shown = allActsShown
                ? recentFirst
                : recentFirst.slice(0, VISIBLE_ACTS)

              return (
                <>
                  <div
                    className={`relative pl-4 border-l border-line space-y-2.5 ${
                      allActsShown ? 'max-h-56 overflow-y-auto pr-2' : ''
                    }`}
                  >
                    {shown.map((acte, i) => (
                      <div key={`${acte.date}-${i}`} className="relative">
                        <div className="absolute -left-[21px] top-1.5 w-2 h-2 rounded-full bg-accent border-2 border-surface" />
                        <p className="text-sm leading-snug">{acte.label}</p>
                        <p className="text-xs text-ink-faint">
                          {new Date(
                            acte.date + 'T00:00:00',
                          ).toLocaleDateString('fr-FR', {
                            day: 'numeric',
                            month: 'long',
                            year: 'numeric',
                          })}
                        </p>
                      </div>
                    ))}
                  </div>

                  {hidden > 0 && (
                    <button
                      type="button"
                      onClick={() => setAllActsShown(!allActsShown)}
                      aria-expanded={allActsShown}
                      className="mt-3 text-xs text-accent hover:underline"
                    >
                      {allActsShown
                        ? 'Masquer les actes antérieurs'
                        : `Afficher les ${hidden} actes antérieurs`}
                    </button>
                  )}
                </>
              )
            })()
          )}
        </Card>
      </div>

      {data.documents.length > 0 && (
        <Card className="mb-6 p-4">
          <SectionTitle>Documents officiels du dossier</SectionTitle>
          <p className="mt-1 text-xs leading-relaxed text-ink-faint">
            Chaque lien ouvre le document identifié dans l’archive de l’Assemblée
            nationale. Ces documents ne sont pas encore associés automatiquement à
            un scrutin : plusieurs versions peuvent exister pour un même dossier.
          </p>
          <ul className="mt-3 divide-y divide-line">
            {data.documents.map((document) => (
              <li key={document.document_uid} className="py-2.5 first:pt-0 last:pb-0">
                <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
                  {document.official_url ? (
                    <a
                      href={document.official_url}
                      target="_blank"
                      rel="noreferrer"
                      className="text-sm font-medium text-ink hover:text-accent hover:underline"
                    >
                      {document.title}
                    </a>
                  ) : (
                    <span className="text-sm font-medium text-ink">{document.title}</span>
                  )}
                  <span className="text-xs text-ink-faint">{document.doc_type}</span>
                  {document.date && (
                    <span className="text-xs text-ink-faint">
                      ·{' '}
                      {new Date(document.date + 'T00:00:00').toLocaleDateString('fr-FR', {
                        day: 'numeric',
                        month: 'long',
                        year: 'numeric',
                      })}
                    </span>
                  )}
                </div>
                <p className="mt-1 text-xs text-ink-faint">
                  Référence officielle : {document.document_uid}
                  {document.source_license && ` · ${document.source_license}`}
                  {document.source_retrieved_at &&
                    ` · relevée le ${new Date(document.source_retrieved_at).toLocaleDateString('fr-FR')}`}
                </p>
              </li>
            ))}
          </ul>
        </Card>
      )}

      <div className="mb-6 border-b border-line" role="tablist" aria-label="Contenu du dossier">
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'actes'}
          onClick={() => selectTab('actes')}
          className={`mr-4 border-b-2 px-1 pb-2 text-sm font-medium ${activeTab === 'actes' ? 'border-accent text-ink' : 'border-transparent text-ink-faint hover:text-ink-soft'}`}
        >
          Avis des groupes
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'tableaux'}
          onClick={() => selectTab('tableaux')}
          className={`border-b-2 px-1 pb-2 text-sm font-medium ${activeTab === 'tableaux' ? 'border-accent text-ink' : 'border-transparent text-ink-faint hover:text-ink-soft'}`}
        >
          Scrutins et amendements
        </button>
      </div>

      {activeTab === 'actes' ? (
        <DossierGroupActions uid={data.uid} />
      ) : (
        <>
          <DossierScrutins uid={data.uid} />
          <DossierAmendments uid={data.uid} initialGroup={searchParams.get('amendements_group') ?? ''} />
        </>
      )}

      <div className="text-center">
        <a
          href={`https://www.assemblee-nationale.fr/dyn/17/dossiers/${data.uid}`}
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs text-ink-faint hover:text-accent underline"
        >
          Voir sur assemblee-nationale.fr ↗
        </a>
      </div>
    </div>
  )
}
