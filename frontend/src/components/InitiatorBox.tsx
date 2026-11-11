import type { ReactNode } from 'react'
import { PoliticalLogo } from './PoliticalLogo'
import { GROUP_LOGOS } from '../content/logos'
import type { InitiatorDto } from '../types/dossiers'


/** « Proposition de loi déposée par », « Projet de loi déposé par ». */
function depositedBy(kind: string) {
  return `${kind} ${kind.startsWith('Proposition') ? 'déposée' : 'déposé'} par`
}

function shortDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/** Vignette d'information de l'en-tête : un visuel à gauche, trois lignes à droite. */
export function InfoBox({ visual, children }: { visual: ReactNode; children: ReactNode }) {
  return (
    <div className="flex items-center gap-3 rounded bg-surface-soft px-3 py-2.5 text-sm leading-snug">
      {visual}
      <div className="min-w-0">{children}</div>
    </div>
  )
}

/**
 * Auteur du texte, avec le logo de son groupe. Le groupe est celui lu à la
 * date du dépôt, et cette date l'accompagne toujours : l'appartenance est
 * datée, et un groupe n'est pas un parti (README.md §3).
 */
export function InitiatorBox({ initiator, kind }: { initiator: InitiatorDto; kind: string }) {
  const group = initiator.reference_date ? initiator.group : null
  return (
    <InfoBox
      visual={
        group ? (
          <PoliticalLogo
            logo={GROUP_LOGOS[group.abbrev]}
            fallback={group.abbrev}
            color={null}
            className="h-11 w-16"
          />
        ) : (
          <span className="flex h-11 w-16 shrink-0 items-center justify-center rounded bg-surface text-xs font-semibold text-ink-soft ring-1 ring-inset ring-line">
            {initiator.role ?? '—'}
          </span>
        )
      }
    >
      <p className="text-xs text-ink-faint">{depositedBy(kind)}</p>
      <p>
        {initiator.official_url ? (
          <a
            href={initiator.official_url}
            target="_blank"
            rel="noreferrer"
            className="font-semibold hover:link"
          >
            {initiator.full_name}
          </a>
        ) : (
          <span className="font-semibold">{initiator.full_name}</span>
        )}
        {group && <span className="text-ink-soft"> · {group.label}</span>}
      </p>
      {group && initiator.reference_date && (
        <p className="text-xs text-ink-faint">
          groupe au {shortDate(initiator.reference_date)}, date du dépôt
        </p>
      )}
    </InfoBox>
  )
}

/**
 * Plusieurs auteurs en une seule vignette : les noms à la suite, chacun avec
 * son groupe au dépôt ; la qualité (ministre, député) passe en infobulle. La
 * date de lecture des groupes reste affichée, une fois par date distincte
 * (README.md §3).
 */
export function InitiatorsBox({ initiators, kind }: { initiators: InitiatorDto[]; kind: string }) {
  const dates = [
    ...new Set(initiators.flatMap((i) => (i.group && i.reference_date ? [i.reference_date] : []))),
  ]
  return (
    <InfoBox
      visual={
        <span className="flex size-11 shrink-0 items-center justify-center rounded bg-surface text-ink-soft ring-1 ring-inset ring-line">
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
            <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h4" />
          </svg>
        </span>
      }
    >
      <p className="text-xs text-ink-faint">{depositedBy(kind)}</p>
      <p>
        {initiators.map((initiator, i) => {
          const group = initiator.reference_date ? initiator.group : null
          return (
            <span key={i}>
              {i > 0 && ', '}
              {initiator.official_url ? (
                <a
                  href={initiator.official_url}
                  target="_blank"
                  rel="noreferrer"
                  title={initiator.role ?? undefined}
                  className="font-semibold hover:link"
                >
                  {initiator.full_name}
                </a>
              ) : (
                <span className="font-semibold" title={initiator.role ?? undefined}>
                  {initiator.full_name}
                </span>
              )}
              {group && (
                <span className="text-ink-soft" title={group.label}>
                  {' '}
                  ({group.abbrev})
                </span>
              )}
            </span>
          )
        })}
      </p>
      {dates.length > 0 && (
        <p className="text-xs text-ink-faint">
          groupe{dates.length > 1 ? 's' : ''} au {dates.map(shortDate).join(', ')}, date du dépôt
        </p>
      )}
    </InfoBox>
  )
}
