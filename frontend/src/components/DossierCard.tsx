import type { ReactNode } from 'react'
import { Link } from 'react-router'
import { FamilyIllustration } from './FamilyIllustration'
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
}

function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/**
 * Bandeau d'état : le sort quand la source en publie un, sinon l'étape qu'elle
 * atteste. Jamais « en cours » : sur la plupart des dossiers, ce serait une
 * affirmation que personne n'a écrite (README.md §6).
 */
function banner(dossier: DossierCardDto): { tone: string; text: string } {
  const { outcome, current_stage: stage } = dossier
  if (outcome.kind !== 'no_recorded_conclusion') {
    const tone =
      outcome.kind === 'promulgated'
        ? 'bg-yes-soft text-yes'
        : outcome.kind === 'rejected'
          ? 'bg-no-soft text-no'
          : 'bg-surface-soft text-ink-soft'
    const date = outcome.date ? ` le ${formatDate(outcome.date)}` : ''
    const law = outcome.law_code ? ` · loi n° ${outcome.law_code}` : ''
    return { tone, text: `${outcome.label}${date}${law}` }
  }
  if (stage) {
    return {
      tone: 'bg-info-soft text-info',
      text: `Étape : ${stage.label}${stage.chamber ? ` — ${stage.chamber}` : ''}`,
    }
  }
  return { tone: 'bg-surface-soft text-ink-soft', text: 'Sans conclusion enregistrée' }
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
        className="inline-flex items-center rounded-md px-1.5 py-px text-[11px] font-bold tracking-wide"
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

function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="min-w-0">
      <dt className="text-[11.5px] text-ink-faint">{label}</dt>
      <dd className="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-1 text-[13.5px] text-ink">
        {children}
      </dd>
    </div>
  )
}

/**
 * Carte d'un dossier dans la liste. Le titre porte le lien, étiré sur toute
 * la carte ; les thèmes restent des boutons à part, qui filtrent la liste.
 */
export default function DossierCard({
  dossier,
  today,
  onFamily,
}: {
  dossier: DossierCardDto
  /** Date du jour (AAAA-MM-JJ) : une étape datée après est annoncée à venir. */
  today: string
  onFamily: (code: string) => void
}) {
  const state = banner(dossier)
  const [first, ...others] = dossier.initiators
  const upcoming = dossier.last_activity_date > today

  return (
    <article className="group relative overflow-hidden rounded-2xl border border-line bg-surface shadow-card transition-colors hover:border-line-strong">
      <div
        className={`flex flex-wrap items-center justify-between gap-x-4 gap-y-0.5 px-5 py-2 text-[13px] font-semibold ${state.tone}`}
      >
        <span>{state.text}</span>
        <span className="text-xs font-medium text-ink-soft">{dossier.procedure}</span>
      </div>

      <div className="px-5 pt-4 pb-5">
        <h2 className="text-[17px] font-semibold leading-snug text-ink transition-colors group-hover:text-accent">
          <Link
            to={`/dossiers/${dossier.uid}`}
            className="after:absolute after:inset-0 focus:outline-none"
          >
            {dossier.title}
          </Link>
        </h2>

        <div className="mt-3 flex flex-wrap gap-1.5">
          {dossier.families.length > 0 ? (
            dossier.families.map((family) => (
              <button
                key={family.code}
                type="button"
                onClick={() => onFamily(family.code)}
                title={`Voir les dossiers du thème « ${family.label} »`}
                className="relative z-10 inline-flex items-center gap-1.5 rounded-lg bg-surface-soft py-0.5 pr-2.5 pl-0.5 text-[13px] font-medium text-ink ring-1 ring-transparent transition hover:ring-line-strong"
              >
                <FamilyIllustration
                  code={family.code}
                  className="size-6 rounded-md"
                  glyphClassName="size-4"
                />
                {family.label}
              </button>
            ))
          ) : (
            <span className="text-xs text-ink-faint italic">
              Aucun thème attribué pour l'instant.
            </span>
          )}
        </div>

        <dl className="mt-4 grid grid-cols-2 gap-x-6 gap-y-3 border-t border-line pt-3 lg:grid-cols-4">
          <Fact label="Déposé par">
            {first ? (
              <>
                <InitiatorTag initiator={first} />
                <span>{first.full_name}</span>
                {others.length > 0 && (
                  <span className="text-ink-faint">
                    et {others.length} autre{others.length > 1 ? 's' : ''}
                  </span>
                )}
              </>
            ) : (
              <span className="text-ink-faint">Non publié</span>
            )}
          </Fact>
          <Fact label="Déposé le">
            {dossier.deposit_date ? (
              formatDate(dossier.deposit_date)
            ) : (
              <span className="text-ink-faint">Non publié</span>
            )}
          </Fact>
          <Fact label={upcoming ? 'Prochaine étape' : 'Dernière étape'}>
            <span>
              {dossier.last_activity_label} · {formatDate(dossier.last_activity_date)}
            </span>
            {upcoming && (
              <span className="rounded bg-info-soft px-1.5 text-[11px] font-semibold text-info">
                à venir
              </span>
            )}
          </Fact>
          <Fact label="Votes">
            {dossier.scrutin_count > 0 ? (
              <span>
                {dossier.scrutin_count.toLocaleString('fr-FR')} scrutin
                {dossier.scrutin_count > 1 ? 's' : ''} public
                {dossier.scrutin_count > 1 ? 's' : ''}
              </span>
            ) : (
              <span className="text-ink-faint">Aucun scrutin public</span>
            )}
          </Fact>
        </dl>
      </div>
    </article>
  )
}
