import { useParams, Link, useSearchParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import DossierAmendments from '../components/DossierAmendments'
import DossierScrutins from '../components/DossierScrutins'
import DossierBrief from '../components/DossierBrief'
import DossierGroupActions from '../components/DossierGroupActions'
import DossierFinalScrutin from '../components/DossierFinalScrutin'
import DossierParcours, {
  type ParcoursActDto,
  type ParcoursDocumentDto,
} from '../components/DossierParcours'
import { FamilyIllustration } from '../components/FamilyIllustration'
import { InfoBox, InitiatorBox, InitiatorsBox } from '../components/InitiatorBox'
import PositionGlyph from '../components/entry/PositionGlyph'
import { FOCUS_RING } from '../components/entry/styles'
import { Card, ErrorPanel, Loading, Pill } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import { typographic } from '../lib/entryFlow'
import { textKind, type InitiatorDto, type OutcomeDto } from '../types/dossiers'

interface StageDto {
  label: string
  chamber: string
}

interface FamilyDto {
  code: string
  label: string
}

interface DossierDetailDto {
  uid: string
  title: string
  procedure: string
  url: string | null
  deposit_date: string | null
  last_activity_date: string
  last_activity_label: string
  acts: ParcoursActDto[]
  documents: ParcoursDocumentDto[]
  current_stage: StageDto | null
  initiators: InitiatorDto[]
  committee: string | null
  outcome: OutcomeDto
  /** Absent d'un serveur antérieur à l'ajout : traité comme vide. */
  families?: FamilyDto[]
}

/** Ce que les onglets lisent pour leur compteur : le total, rien d'autre. */
interface TotalDto {
  total: number
}

type Tab = 'groupes' | 'scrutins' | 'amendements'

function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/**
 * Onglet demandé par l'adresse. « tableaux » est l'ancien onglet commun aux
 * scrutins et aux amendements ; un lien filtré sur un groupe ouvre ses
 * amendements.
 */
function tabFrom(params: URLSearchParams): Tab {
  const tab = params.get('tab')
  if (tab === 'scrutins' || tab === 'amendements') return tab
  if (tab === 'tableaux') return params.get('amendements_group') ? 'amendements' : 'scrutins'
  return 'groupes'
}

/**
 * Bandeau d'issue, comme sur les cartes de la liste : le sort quand la source
 * l'atteste, l'étape publiée sinon. Rien n'est conclu à la place de la source.
 */
function OutcomeBand({ dossier }: { dossier: DossierDetailDto }) {
  const { outcome, current_stage: stage } = dossier
  const step = dossier.last_activity_label.replace(/\.$/, '')
  const lastStep = (
    <span className="text-[12.5px] font-medium text-ink-soft">
      Dernière étape : {step} · {formatDate(dossier.last_activity_date)}
    </span>
  )

  if (outcome.kind === 'promulgated' || outcome.kind === 'rejected') {
    const promulgated = outcome.kind === 'promulgated'
    return (
      <div
        className={`flex flex-wrap items-center justify-between gap-x-4 gap-y-1 px-4 py-2 text-sm font-semibold sm:px-6 ${promulgated ? 'bg-yes-soft text-yes' : 'bg-no-soft text-no'}`}
      >
        <span className="inline-flex items-center gap-1.5">
          <PositionGlyph position={promulgated ? 'for' : 'against'} className="size-3.5" strokeWidth={3.2} />
          {outcome.label}
          {outcome.date && ` le ${formatDate(outcome.date)}`}
          {outcome.law_code && ` · loi n° ${outcome.law_code}`}
        </span>
        {promulgated && (outcome.law_jo_date || outcome.legifrance_url) ? (
          <span className="text-[12.5px] font-medium text-ink-soft">
            {outcome.law_jo_date && `JO du ${formatDate(outcome.law_jo_date)}`}
            {outcome.law_jo_date && outcome.legifrance_url && ' · '}
            {outcome.legifrance_url && (
              <a
                href={outcome.legifrance_url}
                target="_blank"
                rel="noreferrer"
                className="font-semibold text-accent hover:underline"
              >
                Légifrance ↗
              </a>
            )}
          </span>
        ) : (
          lastStep
        )}
      </div>
    )
  }

  const text =
    outcome.kind !== 'no_recorded_conclusion'
      ? `${outcome.label}${outcome.date ? ` le ${formatDate(outcome.date)}` : ''}`
      : stage
        ? `Étape : ${stage.label}${stage.chamber ? ` — ${stage.chamber}` : ''}`
        : 'Sans conclusion enregistrée'
  const tone =
    outcome.kind === 'no_recorded_conclusion' && stage
      ? 'bg-info-soft text-info'
      : 'bg-surface-soft text-ink-soft'
  return (
    <div
      className={`flex flex-wrap items-center justify-between gap-x-4 gap-y-1 px-4 py-2 text-sm font-semibold sm:px-6 ${tone}`}
    >
      <span>
        {text}
        {outcome.kind === 'merged_into' && outcome.merged_into_uid && (
          <Link
            to={`/dossiers/${outcome.merged_into_uid}`}
            className="ml-2 font-medium text-accent hover:underline"
          >
            Voir le dossier →
          </Link>
        )}
      </span>
      {lastStep}
    </div>
  )
}

function CommitteeBox({ committee }: { committee: string }) {
  return (
    <InfoBox
      visual={
        <span className="flex size-11 shrink-0 items-center justify-center rounded-lg bg-surface text-ink-soft ring-1 ring-inset ring-line">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={1.8}
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden
            className="size-6"
          >
            <path d="M3 21h18M4 10h16M12 3l8 5H4zM6 10v8M10 10v8M14 10v8M18 10v8" />
          </svg>
        </span>
      }
    >
      <p className="text-xs text-ink-faint">Commission saisie au fond</p>
      <p className="font-semibold">{committee}</p>
    </InfoBox>
  )
}

function TabButton({
  active,
  count,
  onClick,
  children,
}: {
  active: boolean
  count: number | undefined
  onClick: () => void
  children: string
}) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      onClick={onClick}
      className={`inline-flex items-center gap-2 rounded-[9px] px-3.5 py-1.5 text-sm font-medium transition-colors ${FOCUS_RING} ${
        active ? 'bg-surface text-ink shadow-card' : 'text-ink-soft hover:text-ink'
      }`}
    >
      {children}
      {count !== undefined && (
        <span
          className={`rounded-full px-1.5 text-xs font-semibold tabular-nums ${active ? 'bg-ink text-canvas' : 'bg-line text-ink-soft'}`}
        >
          {count.toLocaleString('fr-FR')}
        </span>
      )}
    </button>
  )
}

/**
 * Fiche d'un dossier, en lecture seule : l'issue et le titre en tête, les
 * actes des groupes, scrutins et amendements en onglets, et dans la colonne de
 * droite qui a déposé le texte et le chemin qu'il a suivi.
 */
export default function DossierDetailPage() {
  const { uid } = useParams<{ uid: string }>()
  const [searchParams, setSearchParams] = useSearchParams()

  const { data, isLoading, isError, error } = useQuery<DossierDetailDto>({
    queryKey: ['dossier', uid],
    queryFn: () => fetchJson(`/api/dossiers/${uid}`),
    enabled: !!uid,
  })

  // Compteurs des onglets : une page d'un seul élément suffit à lire le total.
  const { data: scrutinTotal } = useQuery<TotalDto>({
    queryKey: ['dossier-scrutins-total', uid],
    queryFn: () => fetchJson(`/api/dossiers/${uid}/scrutins?limit=1`),
    enabled: !!uid,
  })
  const { data: amendmentTotal } = useQuery<TotalDto>({
    queryKey: ['dossier-amendements-total', uid],
    queryFn: () => fetchJson(`/api/dossiers/${uid}/amendements?limit=1`),
    enabled: !!uid,
  })

  if (isLoading) return <Loading>Chargement du dossier…</Loading>

  if (isError) {
    return (
      <div>
        <Link to="/" className="mb-4 inline-block text-sm font-medium text-accent hover:underline">
          ← Retour aux dossiers
        </Link>
        <ErrorPanel error={error} />
      </div>
    )
  }

  if (!data) return null

  const activeTab = tabFrom(searchParams)
  const selectTab = (tab: Tab) => {
    const next = new URLSearchParams(searchParams)
    if (tab === 'groupes') next.delete('tab')
    else next.set('tab', tab)
    if (tab !== 'amendements') next.delete('amendements_group')
    setSearchParams(next, { preventScrollReset: true })
  }
  const families = data.families ?? []
  const kind = textKind(data.procedure)

  return (
    <div>
      <Link to="/" className="mb-4 inline-block text-sm font-medium text-accent hover:underline">
        ← Retour aux dossiers
      </Link>

      <div className="grid items-start gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="min-w-0">
          <Card className="mb-4 overflow-hidden">
            <OutcomeBand dossier={data} />
            <div className="px-4 py-4 sm:px-6 sm:py-5">
              <div className="flex flex-wrap items-center gap-2 text-[13px] text-ink-soft">
                <Pill tone="info">{data.procedure.replace(/ ordinaire$/, '')}</Pill>
                {families.length > 0 ? (
                  families.map((family) => (
                    <Link
                      key={family.code}
                      to={`/themes/${family.code}`}
                      className={`inline-flex items-center gap-2 rounded-full border border-line bg-surface py-0.5 pr-3 pl-0.5 text-[13px] font-semibold text-ink transition-colors hover:border-line-strong ${FOCUS_RING}`}
                    >
                      <FamilyIllustration
                        code={family.code}
                        className="size-6 shrink-0 rounded-full"
                        glyphClassName="size-3.5"
                      />
                      {typographic(family.label)}
                    </Link>
                  ))
                ) : (
                  <span className="text-xs text-ink-faint italic">Aucun thème attribué pour l'instant.</span>
                )}
              </div>
              <h2 className="mt-3 max-w-4xl text-2xl leading-tight font-semibold tracking-tight sm:text-[26px]">
                {data.title}
              </h2>
              <a
                href={data.url ?? `https://www.assemblee-nationale.fr/dyn/17/dossiers/${data.uid}`}
                target="_blank"
                rel="noopener noreferrer"
                className="mt-3 inline-block text-sm font-medium text-accent hover:underline"
              >
                Dossier sur assemblee-nationale.fr ↗
              </a>
            </div>
          </Card>

          <DossierBrief uid={data.uid} />

          <DossierFinalScrutin uid={data.uid} />

          <div
            role="tablist"
            aria-label="Contenu du dossier"
            className="mb-4 inline-flex flex-wrap gap-1 rounded-xl bg-surface-soft p-1 ring-1 ring-inset ring-line"
          >
            <TabButton active={activeTab === 'groupes'} count={undefined} onClick={() => selectTab('groupes')}>
              Groupes
            </TabButton>
            <TabButton
              active={activeTab === 'scrutins'}
              count={scrutinTotal?.total}
              onClick={() => selectTab('scrutins')}
            >
              Scrutins publics
            </TabButton>
            <TabButton
              active={activeTab === 'amendements'}
              count={amendmentTotal?.total}
              onClick={() => selectTab('amendements')}
            >
              Amendements
            </TabButton>
          </div>

          {activeTab === 'groupes' && <DossierGroupActions uid={data.uid} />}
          {activeTab === 'scrutins' && <DossierScrutins uid={data.uid} />}
          {activeTab === 'amendements' && (
            <DossierAmendments uid={data.uid} initialGroup={searchParams.get('amendements_group') ?? ''} />
          )}
        </div>

        <aside className="space-y-4">
          {(data.initiators.length > 0 || data.committee) && (
            <Card className="space-y-1 p-2">
              {data.initiators.length === 1 ? (
                <InitiatorBox initiator={data.initiators[0]} kind={kind} />
              ) : data.initiators.length > 1 ? (
                <InitiatorsBox initiators={data.initiators} kind={kind} />
              ) : null}
              {data.committee && <CommitteeBox committee={data.committee} />}
            </Card>
          )}

          <Card className="px-5 py-4">
            <h3 className="mb-3 text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
              Parcours du texte
            </h3>
            <DossierParcours acts={data.acts} documents={data.documents} />
          </Card>
        </aside>
      </div>
    </div>
  )
}
