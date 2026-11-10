import DossierCardShell, { OutcomeSplit } from './DossierCardShell'
import { Icon } from './icons'
import { formatShortDate } from '../lib/dossierStatus'
import type { OutcomeDto } from '../types/dossiers'
import type { AssignedFamilyDto } from '../types/themes'

export interface DossierCardInitiatorDto {
  full_name: string
  /** `deputy`, `minister`, `senator`, `other` ; absent si non résolu. */
  role: string | null
  group_abbrev: string | null
  group_label: string | null
  group_color: string | null
  group_text_tone: string | null
}

export interface DossierCardDto {
  uid: string
  title: string
  procedure: string
  deposit_date: string | null
  last_activity_date: string
  last_activity_label: string
  current_stage: { label: string; chamber: string } | null
  outcome: OutcomeDto
  initiators: DossierCardInitiatorDto[]
  families: AssignedFamilyDto[]
  scrutin_count: number
  /** Adoptés et rejetés selon la source : leur somme fait `scrutin_count`. */
  adopted_count: number
  rejected_count: number
  /** Absent : aucun scrutin public. */
  last_scrutin_date: string | null
}

/** Réponse de `GET /api/dossiers`. */
export interface DossierPageResponse {
  page: number
  per_page: number
  total: number
  total_pages: number
  dossiers: DossierCardDto[]
}

/**
 * Pastille de l'initiateur : le groupe du dépôt aux couleurs qu'il publie, ou
 * l'institution pour un ministre ou un sénateur. Sans couleur publiée, le
 * sigle reste en texte.
 */
function InitiatorTag({ initiator }: { initiator: DossierCardInitiatorDto }) {
  const neutral =
    'inline-flex items-center rounded-md bg-surface-soft px-1.5 py-px text-[11px] font-semibold text-ink-soft ring-1 ring-inset ring-line'
  if (initiator.group_abbrev && initiator.group_color) {
    return (
      <span
        title={initiator.group_label ?? undefined}
        className="inline-flex items-center rounded-md px-1.5 py-px text-[11px] font-bold tracking-wide ring-1 ring-inset ring-badge-edge"
        style={{
          backgroundColor: initiator.group_color,
          color: initiator.group_text_tone === 'white' ? '#ffffff' : '#000000',
        }}
      >
        {initiator.group_abbrev}
      </span>
    )
  }
  if (initiator.group_abbrev) {
    return (
      <span title={initiator.group_label ?? undefined} className={neutral}>
        {initiator.group_abbrev}
      </span>
    )
  }
  if (initiator.role === 'minister') return <span className={neutral}>Gouvernement</span>
  if (initiator.role === 'senator') return <span className={neutral}>Sénat</span>
  return null
}

/**
 * Carte d'un dossier dans la liste. Le titre porte le lien, étiré sur toute
 * la carte ; les sujets restent des liens à part, vers le parcours par sujet.
 * Le bandeau dit où en est le dossier, la rangée du bas qui l'a déposé.
 * Avec la répartition, le nombre de scrutins passe dans sa portée ; sans
 * elle, il reste dans la rangée du bas, s'il y en a.
 */
export default function DossierCard({
  dossier,
  today,
  onFamily,
  showSplit = false,
}: {
  dossier: DossierCardDto
  /** Date du jour (AAAA-MM-JJ) : une étape datée après est annoncée à venir. */
  today: string
  /** Sans lui, une puce de sujet mène au parcours par sujet. */
  onFamily?: (code: string) => void
  /**
   * Répartition adoptés / rejetés sous les sujets ; à sa place, pour un texte
   * jamais voté, la mention qu'il n'a eu aucun vote public.
   */
  showSplit?: boolean
}) {
  const [first, ...others] = dossier.initiators
  const split = showSplit
  const voted = dossier.scrutin_count > 0

  const meta = (
    <div
      className={`flex flex-wrap items-center gap-x-1.5 gap-y-1 text-[13px] text-ink-soft ${split ? 'mt-2.5' : ''}`}
    >
      {first ? (
        <>
          <InitiatorTag initiator={first} />
          <span className="text-ink">{first.full_name}</span>
          {others.length > 0 && (
            <span className="text-ink-faint">
              et {others.length} autre{others.length > 1 ? 's' : ''}
            </span>
          )}
        </>
      ) : (
        <span className="text-ink-faint">Auteur non publié</span>
      )}
      {dossier.deposit_date && (
        <>
          <span className="text-line-strong">·</span>
          <span>déposé le {formatShortDate(dossier.deposit_date, today)}</span>
        </>
      )}
      {!split && voted && (
        <>
          <span className="text-line-strong">·</span>
          <span className="font-semibold text-ink">
            {dossier.scrutin_count.toLocaleString('fr-FR')} vote
            {dossier.scrutin_count > 1 ? 's' : ''}
          </span>
        </>
      )}
    </div>
  )

  return (
    <DossierCardShell
      uid={dossier.uid}
      title={dossier.title}
      status={dossier}
      today={today}
      families={dossier.families}
      onFamily={onFamily}
      stretched
      themesAside={split ? undefined : meta}
      split={
        split &&
        (voted ? (
          <OutcomeSplit
            adopted={dossier.adopted_count}
            rejected={dossier.rejected_count}
            scope={`${dossier.scrutin_count.toLocaleString('fr-FR')} vote${dossier.scrutin_count > 1 ? 's' : ''}`}
          />
        ) : (
          <p className="mt-2.5 flex items-center gap-1.5 text-[12.5px] text-ink-faint">
            <Icon name="info" className="size-4 shrink-0" />
            Aucun vote public à l'Assemblée
          </p>
        ))
      }
      below={split ? meta : undefined}
    />
  )
}
