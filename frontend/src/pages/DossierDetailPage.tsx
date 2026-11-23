import { useParams, Link, useSearchParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { Tabs } from '@base-ui/react/tabs'
import DossierAmendments from '../components/DossierAmendments'
import DossierScrutins from '../components/DossierScrutins'
import DossierBrief from '../components/DossierBrief'
import DossierGroupActions from '../components/DossierGroupActions'
import DossierFinalScrutin from '../components/DossierFinalScrutin'
import { InfoTip } from '../components/InfoTip'
import LawAnswer from '../components/LawAnswer'
import LawMilestones from '../components/LawMilestones'
import LawStateBadge from '../components/LawStateBadge'
import DossierParcours, {
  DocumentsSource,
  type ParcoursActDto,
  type ParcoursDocumentDto,
} from '../components/DossierParcours'
import { FamilyIllustration } from '../components/FamilyIllustration'
import { InfoBox, InitiatorBox, InitiatorsBox } from '../components/InitiatorBox'
import { StateDot } from '../components/entry/PositionGlyph'
import { FOCUS_RING } from '../components/entry/styles'
import { FOCUS } from '../components/SheetControls'
import { BackLink, ErrorPanel, Loading } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import { REJECTION_NOT_FINAL, readingKey, readingLabel } from '../lib/dossierStatus'
import { themeLink, typographic } from '../lib/entryFlow'
import {
  textKind,
  type InitiatorDto,
  type LawProgress,
  type OutcomeDto,
} from '../types/dossiers'

interface StageDto {
  label: string
  chamber: string
}

interface FamilyDto {
  code: string
  label: string
}

/** Réponse de `/api/dossiers/{uid}`, avec état, jalons et réponse (SPEC-navette). */
interface DossierDetailDto extends LawProgress {
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

function capitalize(text: string) {
  return text.charAt(0).toUpperCase() + text.slice(1)
}

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
 * Issue d'un dossier sans état « où en est la loi », comme sur les cartes de la
 * liste : le sort quand la source l'atteste, l'étape publiée sinon. Rien n'est
 * conclu à la place de la source. Refonte A : une ligne à plat en tête de la
 * fiche, plus de bandeau teinté ; le vert reste à la promulgation.
 */
function OutcomeBand({ dossier }: { dossier: DossierDetailDto }) {
  const { outcome, current_stage: stage } = dossier
  const step = dossier.last_activity_label.replace(/\.$/, '')
  // Une étape datée après aujourd'hui n'a pas encore eu lieu : comme sur la carte.
  const upcoming = dossier.last_activity_date > new Date().toISOString().slice(0, 10)
  const lastStep = (
    <span className="text-[13px] font-medium text-ink-soft">
      {upcoming ? 'Prochaine étape' : 'Dernière étape'} : {step} ·{' '}
      {formatDate(dossier.last_activity_date)}
      {upcoming && (
        <span className="ml-1.5 rounded bg-surface px-1.5 text-[11px] font-semibold text-info">
          à venir
        </span>
      )}
    </span>
  )
  const line = 'mb-4 flex flex-wrap items-center justify-between gap-x-4 gap-y-1 text-[13px] leading-[18px]'

  if (outcome.kind === 'rejected') {
    // La lecture vient de l'acte de décision du jour du rejet : « Assemblée ·
    // 1re lecture ». Sans acte daté, le libellé de la source suffit.
    const decision = [...dossier.acts]
      .reverse()
      .find((act) => act.date === outcome.date && act.code?.endsWith('-DEC'))
    const reading = decision ? readingLabel(readingKey(decision.code)) : undefined
    return (
      <div className={line}>
        <span className="font-semibold text-ink">
          {capitalize(outcome.label)}
          {reading && ` · ${reading}`}
          {outcome.date && ` · ${formatDate(outcome.date)}`}
          <span className="font-medium"> — {REJECTION_NOT_FINAL}</span>
        </span>
        {lastStep}
      </div>
    )
  }

  if (outcome.kind === 'promulgated') {
    return (
      <div className={line}>
        <span className="inline-flex items-center gap-1.5 font-semibold text-yes">
          <StateDot position="for" />
          {outcome.label}
          {outcome.date && ` le ${formatDate(outcome.date)}`}
          {outcome.law_code && ` · loi n° ${outcome.law_code}`}
        </span>
        {outcome.law_jo_date || outcome.legifrance_url ? (
          <span className="text-[13px] font-medium text-ink-soft">
            {outcome.law_jo_date && `JO du ${formatDate(outcome.law_jo_date)}`}
            {outcome.law_jo_date && outcome.legifrance_url && ' · '}
            {outcome.legifrance_url && (
              <a
                href={outcome.legifrance_url}
                target="_blank"
                rel="noreferrer"
                className="font-semibold link"
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
        : 'Sans conclusion à ce jour'
  const tone =
    outcome.kind === 'no_recorded_conclusion' && stage ? 'text-ink' : 'text-ink-soft'
  return (
    <div className={line}>
      <span className={`font-semibold ${tone}`}>
        {text}
        {outcome.kind === 'merged_into' && outcome.merged_into_uid && (
          <Link
            to={`/dossiers/${outcome.merged_into_uid}`}
            className="ml-2 font-medium link"
          >
            Voir le texte →
          </Link>
        )}
      </span>
      {lastStep}
    </div>
  )
}

function CommitteeBox({ committee }: { committee: string }) {
  return (
    <InfoBox flat visual={null}>
      <p className="text-xs text-ink-faint">Commission saisie au fond</p>
      <p className="font-semibold">{committee}</p>
    </InfoBox>
  )
}

/**
 * Onglet souligné (refonte A) : 2 px d'accent sous l'onglet actif, texte d'encre ;
 * les autres en gris. Sur téléphone, les trois onglets se partagent la largeur et
 * le compteur passe sous le mot ; au-delà, il le suit.
 */
function TabButton({
  value,
  active,
  count,
  children,
}: {
  value: Tab
  active: boolean
  count: number | undefined
  children: string
}) {
  return (
    <Tabs.Tab
      value={value}
      className={`-mb-px flex min-h-14 min-w-0 flex-col items-start justify-start gap-px border-b-2 pt-3 pb-1.5 text-left text-sm font-semibold whitespace-nowrap transition-colors sm:min-h-12 sm:flex-row sm:items-center sm:gap-2 sm:pt-0 sm:pb-0 ${FOCUS} ${
        active ? 'border-accent text-ink' : 'border-transparent text-ink-faint hover:text-ink'
      }`}
    >
      {children}
      {count !== undefined && (
        <span className="text-[12.5px] font-medium text-ink-faint tabular-nums">
          {count.toLocaleString('fr-FR')}
        </span>
      )}
    </Tabs.Tab>
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

  if (isLoading) return <Loading>Chargement du texte…</Loading>

  if (isError) {
    return (
      <div>
        <BackLink to="/scrutins?vue=texte" className="mb-4">Retour aux textes</BackLink>
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
  const lawState = data.law_state

  const dossierLink = (
    <a
      href={data.url ?? `https://www.assemblee-nationale.fr/dyn/17/dossiers/${data.uid}`}
      target="_blank"
      rel="noopener noreferrer"
      className="inline-block text-sm font-semibold link"
    >
      Dossier législatif sur assemblee-nationale.fr ↗
    </a>
  )

  return (
    <div>
      <BackLink to="/scrutins?vue=texte" className="mb-4">Retour aux textes</BackLink>

      {/* Les trois états (SPEC-navette) remplacent le bandeau d'issue ; un
          dossier sans état garde la ligne d'issue (RM-04). */}
      {lawState && (
        <LawStateBadge kind={lawState.kind} variant="line" uid={data.uid} className="mt-2 mb-3" />
      )}

      <div className="grid grid-cols-[minmax(0,1fr)] items-start gap-x-10 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="min-w-0">
          <header>
            {!lawState && <OutcomeBand dossier={data} />}
            <div className="flex flex-wrap items-center gap-2 text-[13px] text-ink-soft">
              <span className="inline-flex h-[26px] items-center rounded-full bg-canvas px-2.5 text-[13px] font-medium text-ink-soft ring-1 ring-line ring-inset">
                {data.procedure.replace(/ ordinaire$/, '')}
              </span>
              {families.length > 0 ? (
                families.map((family) => (
                  <Link
                    key={family.code}
                    to={themeLink(family.code)}
                    className={`inline-flex h-[26px] items-center gap-1.5 rounded-full bg-surface pr-2.5 pl-2 text-[13px] font-medium text-ink-soft transition-colors hover:bg-surface-soft hover:text-ink ${FOCUS_RING}`}
                  >
                    <FamilyIllustration code={family.code} bare className="shrink-0" glyphClassName="size-[15px]" />
                    {typographic(family.label)}
                  </Link>
                ))
              ) : (
                <span className="text-xs text-ink-faint italic">Aucun sujet attribué pour l'instant.</span>
              )}
            </div>
            <h2 className="mt-3.5 max-w-4xl heading-2 text-balance max-md:text-[30px] max-md:leading-[33px] md:heading-1">
              {data.title}
            </h2>
            {!lawState && <p className="mt-3.5">{dossierLink}</p>}
          </header>

          {lawState && (
            <section className="mt-8 py-7 max-lg:band lg:rounded lg:bg-surface lg:p-7">
              <LawMilestones milestones={data.milestones} stateKind={lawState.kind} />
              {data.answer && (
                <LawAnswer answer={data.answer} lawState={lawState} compact className="mt-5" />
              )}
              <p className="mt-4">{dossierLink}</p>
            </section>
          )}

          <DossierBrief uid={data.uid} />

          <DossierFinalScrutin uid={data.uid} acts={data.acts} />

          {/* Chaque onglet charge sa liste : les flèches déplacent le focus,
              Entrée ou Espace l'activent. */}
          <Tabs.Root value={activeTab} onValueChange={(tab: Tab) => selectTab(tab)} className="mt-9 md:mt-12">
            <Tabs.List
              aria-label="Contenu du texte"
              className="mb-6 grid grid-cols-3 gap-x-4 border-b border-line sm:flex sm:gap-x-7"
            >
              <TabButton value="groupes" active={activeTab === 'groupes'} count={undefined}>
                Groupes
              </TabButton>
              <TabButton value="scrutins" active={activeTab === 'scrutins'} count={scrutinTotal?.total}>
                Votes publics
              </TabButton>
              <TabButton
                value="amendements"
                active={activeTab === 'amendements'}
                count={amendmentTotal?.total}
              >
                Amendements
              </TabButton>
            </Tabs.List>

            {activeTab === 'groupes' && <DossierGroupActions uid={data.uid} />}
            {activeTab === 'scrutins' && <DossierScrutins uid={data.uid} />}
            {activeTab === 'amendements' && (
              <DossierAmendments uid={data.uid} initialGroup={searchParams.get('amendements_group') ?? ''} />
            )}
          </Tabs.Root>
        </div>

        <aside className="mt-12 lg:mt-0 lg:rounded lg:bg-surface lg:p-6">
          {(data.initiators.length > 0 || data.committee) && (
            <div className="space-y-4 lg:border-b lg:border-line lg:pb-6">
              {data.initiators.length === 1 ? (
                <InitiatorBox initiator={data.initiators[0]} kind={kind} flat />
              ) : data.initiators.length > 1 ? (
                <InitiatorsBox initiators={data.initiators} kind={kind} flat />
              ) : null}
              {data.committee && <CommitteeBox committee={data.committee} />}
            </div>
          )}

          <section className="mt-9 py-8 max-lg:band lg:mt-0 lg:pt-6 lg:pb-0">
            <h3 className="mb-4 heading-4">
              Parcours du texte
              {data.documents.length > 0 && (
                <InfoTip label="Source des documents" className="md:hidden">
                  <DocumentsSource documents={data.documents} />
                </InfoTip>
              )}
            </h3>
            <DossierParcours acts={data.acts} documents={data.documents} />
          </section>
        </aside>
      </div>
    </div>
  )
}
