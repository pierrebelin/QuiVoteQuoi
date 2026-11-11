import type { OutcomeDto, OutcomeKind } from '../types/dossiers'
import { Card, Pill, type PillTone } from './ui'

const tones: Record<OutcomeKind, PillTone> = {
  promulgated: 'yes',
  rejected: 'no',
  withdrawn: 'neutral',
  merged_into: 'neutral',
  no_recorded_conclusion: 'neutral',
}

function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/**
 * Pastille de sort du dossier.
 *
 * Rien n'est affiché quand la source ne conclut rien : sur les neuf dixièmes
 * des dossiers, une pastille « en cours » serait une affirmation que personne
 * n'a écrite. Le dernier acte, déjà présent sur la carte, dit ce qui est su.
 */
export default function OutcomeBadge({ outcome }: { outcome: OutcomeDto }) {
  if (outcome.kind === 'no_recorded_conclusion') return null

  return (
    <Pill tone={tones[outcome.kind]}>
      {outcome.label}
      {outcome.date && ` — ${formatDate(outcome.date)}`}
    </Pill>
  )
}

/**
 * Bloc détaillé, pour la page d'un dossier. Contrairement à la pastille, il
 * dit explicitement quand la source ne conclut rien plutôt que de se taire.
 */
export function OutcomePanel({
  outcome,
  lastActivityLabel,
  lastActivityDate,
}: {
  outcome: OutcomeDto
  lastActivityLabel: string
  lastActivityDate: string
}) {
  if (outcome.kind === 'no_recorded_conclusion') {
    return (
      <Card className="px-4 py-3">
        <p className="text-sm font-semibold">Sans conclusion à ce jour</p>
        <p className="text-xs text-ink-soft mt-0.5">
          L'Assemblée n'a publié aucun acte de clôture pour ce texte. Dernier
          acte connu : {lastActivityLabel}, le {formatDate(lastActivityDate)}.
        </p>
      </Card>
    )
  }

  return (
    <Card className="px-4 py-3">
      <div className="flex items-center gap-2">
        <OutcomeBadge outcome={outcome} />
        {!outcome.is_final && (
          <span className="text-xs text-ink-faint">
            la navette peut reprendre
          </span>
        )}
      </div>

      {outcome.law_code && (
        <p className="text-xs text-ink-soft mt-1.5">
          Loi n° {outcome.law_code}
          {outcome.law_jo_date &&
            `, publiée au Journal officiel le ${formatDate(outcome.law_jo_date)}`}
          {outcome.legifrance_url && (
            <>
              {' — '}
              <a
                href={outcome.legifrance_url}
                target="_blank"
                rel="noreferrer"
                className="link"
              >
                texte sur Légifrance
              </a>
            </>
          )}
        </p>
      )}

      {outcome.merged_into_uid && (
        <p className="text-xs text-ink-soft mt-1.5">
          Suite de l'examen dans le texte{' '}
          <a
            href={`/dossiers/${outcome.merged_into_uid}`}
            className="link"
          >
            {outcome.merged_into_uid}
          </a>
        </p>
      )}
    </Card>
  )
}
