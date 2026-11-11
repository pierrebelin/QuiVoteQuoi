import { useState } from 'react'

export interface ParcoursActDto {
  date: string
  label: string
  /** Code publié par l'Assemblée : « AN1-DEPOT », « SN1-COM-FOND-RAPPORT »… */
  code?: string | null
}

export interface ParcoursDocumentDto {
  document_uid: string
  title: string
  doc_type: string
  date: string | null
  official_url: string | null
  source_license: string | null
  source_retrieved_at: string | null
}

/** Phases au-delà desquelles le parcours se replie : un texte budgétaire en compte des dizaines. */
const VISIBLE_PHASES = 4

/**
 * Lecture désignée par le préfixe du code d'acte. Le libellé n'est qu'un nom
 * de lecture : il ne dit rien du sort, que seuls les actes publiés portent.
 */
const PHASE_LABELS: Record<string, string> = {
  AN1: 'Assemblée · 1re lecture',
  SN1: 'Sénat · 1re lecture',
  AN2: 'Assemblée · 2e lecture',
  SN2: 'Sénat · 2e lecture',
  AN3: 'Assemblée · 3e lecture',
  SN3: 'Sénat · 3e lecture',
  CMP: 'Commission mixte paritaire',
  ANNLEC: 'Assemblée · nouvelle lecture',
  SNNLEC: 'Sénat · nouvelle lecture',
  ANLDEF: 'Assemblée · lecture définitive',
  CC: 'Conseil constitutionnel',
  PROM: 'Promulgation',
}

interface Step {
  label: string
  first: string
  last: string
  count: number
}

interface Phase {
  key: string
  label: string
  chamber: 'AN' | 'SN' | null
  first: string
  last: string
  steps: Step[]
  documents: ParcoursDocumentDto[]
}

function phaseKey(code: string | null | undefined) {
  return (code ?? '').split('-')[0]
}

function chamberOf(key: string): 'AN' | 'SN' | null {
  if (key.startsWith('AN')) return 'AN'
  if (key.startsWith('SN')) return 'SN'
  return null
}

/** Chambre d'un document, lue dans sa référence (« PIONANR5… », « RAPPSNR5… »). */
function documentChamber(uid: string): 'AN' | 'SN' | null {
  const chamber = uid.slice(4, 6)
  return chamber === 'AN' || chamber === 'SN' ? chamber : null
}

/** Numéro publié du document, lu dans sa référence (« …B0949 » → 949). */
function documentNumber(uid: string): string | null {
  const match = /B0*(\d+)$/.exec(uid)
  return match ? match[1] : null
}

function shortDate(iso: string, withYear = true) {
  return new Date(iso.slice(0, 10) + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    ...(withYear ? { year: 'numeric' } : {}),
  })
}

function range(first: string, last: string) {
  if (first === last) return shortDate(first)
  const sameYear = first.slice(0, 4) === last.slice(0, 4)
  return `${shortDate(first, !sameYear)} → ${shortDate(last)}`
}

/**
 * Actes rangés par lecture, dans l'ordre de la source : une lecture est une
 * suite d'actes au même préfixe de code. Deux actes successifs de même libellé
 * ne font qu'une étape, comptée. Chaque document rejoint la dernière lecture
 * de sa chambre ouverte à sa date.
 */
function buildPhases(acts: ParcoursActDto[], documents: ParcoursDocumentDto[]) {
  const phases: Phase[] = []
  for (const act of acts) {
    const key = phaseKey(act.code)
    let phase = phases[phases.length - 1]
    if (!phase || phase.key !== key) {
      phase = {
        key,
        label: PHASE_LABELS[key] ?? 'Autres actes',
        chamber: chamberOf(key),
        first: act.date,
        last: act.date,
        steps: [],
        documents: [],
      }
      phases.push(phase)
    }
    phase.last = act.date
    const label = act.label.replace(/\.$/, '')
    const step = phase.steps[phase.steps.length - 1]
    if (step && step.label === label) {
      step.last = act.date
      step.count += 1
    } else {
      phase.steps.push({ label, first: act.date, last: act.date, count: 1 })
    }
  }

  const unplaced: ParcoursDocumentDto[] = []
  for (const document of documents) {
    const chamber = documentChamber(document.document_uid)
    const home = document.date
      ? [...phases]
          .reverse()
          .find((phase) => phase.chamber === chamber && phase.first <= document.date!.slice(0, 10))
      : undefined
    if (home) home.documents.push(document)
    else unplaced.push(document)
  }
  return { phases, unplaced }
}

function DocumentLink({ document }: { document: ParcoursDocumentDto }) {
  const number = documentNumber(document.document_uid)
  const label = number ? `${document.doc_type} n° ${number}` : document.doc_type
  const title = `${document.title}\nRéférence officielle : ${document.document_uid}`
  return document.official_url ? (
    <a
      href={document.official_url}
      target="_blank"
      rel="noreferrer"
      title={title}
      className="font-medium link"
    >
      {label} ↗
    </a>
  ) : (
    <span title={title} className="font-medium text-ink-soft">
      {label}
    </span>
  )
}

/**
 * Parcours du texte, en frise verticale : une pastille par lecture, ses étapes
 * datées dessous, et les documents officiels rangés dans la lecture qui les a
 * produits.
 */
/** Provenance des documents ; sur téléphone, dans l'infobulle du titre du parcours. */
export function DocumentsSource({ documents }: { documents: ParcoursDocumentDto[] }) {
  const retrieved = documents.find((d) => d.source_retrieved_at)?.source_retrieved_at
  const license = documents.find((d) => d.source_license)?.source_license
  return (
    <>
      Documents : archive de l'Assemblée nationale
      {license && `, ${license}`}
      {retrieved && `, relevés le ${new Date(retrieved).toLocaleDateString('fr-FR')}`}.{' '}
      {/* Au doigt, pas de survol : la référence reste sur la page officielle du lien. */}
      <span className="[@media(hover:none)]:hidden">Survolez un lien pour sa référence officielle.</span>
    </>
  )
}

export default function DossierParcours({
  acts,
  documents,
}: {
  acts: ParcoursActDto[]
  documents: ParcoursDocumentDto[]
}) {
  const [expanded, setExpanded] = useState(false)
  const { phases, unplaced } = buildPhases(acts, documents)

  if (phases.length === 0 && unplaced.length === 0) {
    return <p className="text-sm text-ink-faint">Aucun acte enregistré.</p>
  }

  // Les lectures les plus récentes disent où en est le texte : repliées,
  // ce sont elles qui restent visibles.
  const hidden = expanded ? 0 : Math.max(0, phases.length - VISIBLE_PHASES)
  const shown = phases.slice(hidden)

  return (
    <div>
      {hidden > 0 && (
        <button
          type="button"
          onClick={() => setExpanded(true)}
          className="mb-3 text-xs font-medium link"
        >
          Afficher {hidden} lecture{hidden > 1 ? 's' : ''} antérieure{hidden > 1 ? 's' : ''}
        </button>
      )}
      <ol className="ml-1.5 border-l-2 border-line pl-5">
        {shown.map((phase, i) => {
          const isLast = i === shown.length - 1
          return (
            <li key={`${phase.key}-${phase.first}-${i}`} className={isLast ? '' : 'pb-4'}>
              <div className="relative">
                <span
                  aria-hidden
                  className={`absolute top-[5px] -left-[28px] size-3 rounded-full ring-4 ring-surface ${phase.key === 'PROM' ? 'bg-yes' : isLast ? 'bg-info' : 'bg-ink-faint'}`}
                />
                <p className="text-sm leading-snug font-semibold text-ink">{phase.label}</p>
                <p className="text-xs text-ink-faint">{range(phase.first, phase.last)}</p>
              </div>
              <ul className="mt-2 space-y-2 text-[13px] leading-snug">
                {phase.steps.map((step, j) => (
                  <li key={j} className="relative">
                    <span
                      aria-hidden
                      className="absolute top-[6px] -left-[25px] size-1.5 rounded-full bg-line-strong"
                    />
                    <span className="text-ink">
                      {step.label}
                      {step.count > 1 && (
                        <span className="ml-1 rounded bg-surface-soft px-1 text-[11px] font-semibold text-ink-soft">
                          ×{step.count}
                        </span>
                      )}
                    </span>
                    <span className="block text-xs text-ink-faint">{range(step.first, step.last)}</span>
                  </li>
                ))}
              </ul>
              {phase.documents.length > 0 && (
                <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-[13px]">
                  {phase.documents.map((document) => (
                    <DocumentLink key={document.document_uid} document={document} />
                  ))}
                </div>
              )}
            </li>
          )
        })}
      </ol>

      {unplaced.length > 0 && (
        <div className="mt-4 border-t border-line pt-3">
          <p className="mb-1 text-xs text-ink-faint">Autres documents</p>
          <div className="flex flex-col gap-1 text-[13px]">
            {unplaced.map((document) => (
              <DocumentLink key={document.document_uid} document={document} />
            ))}
          </div>
        </div>
      )}

      {documents.length > 0 && (
        <p className="mt-4 hidden text-xs leading-relaxed text-ink-faint md:block">
          <DocumentsSource documents={documents} />
        </p>
      )}
    </div>
  )
}
