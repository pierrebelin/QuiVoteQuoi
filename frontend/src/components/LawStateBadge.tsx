import { LAW_STATE_LABELS, LAW_STATE_TONES } from '../lib/lawState'
import type { LawStateKind } from '../types/dossiers'

/**
 * Icône d'un état : flèches de la navette, horloge de la promulgation
 * attendue, coche de la loi. Elle suit la couleur du texte qui l'entoure.
 */
export function LawStateIcon({
  kind,
  className = 'size-3',
  bold = false,
}: {
  kind: LawStateKind
  className?: string
  /** Trait épaissi pour les petites tailles (jalons). */
  bold?: boolean
}) {
  const width = kind === 'became_law' ? 3.2 : 2.8
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={bold ? width + 0.8 : width}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={`shrink-0 ${className}`}
    >
      {kind === 'in_shuttle' && (
        <>
          <path d="M4 8h14l-4-4" />
          <path d="M20 16H6l4 4" />
        </>
      )}
      {kind === 'definitively_adopted' && (
        <>
          <circle cx="12" cy="12" r="9" />
          <path d="M12 7v5l3 2" />
        </>
      )}
      {kind === 'became_law' && <polyline points="20 6 9 17 4 12" />}
    </svg>
  )
}

/**
 * Étiquette d'état d'un dossier (En navette, Adopté définitivement, Devenu
 * loi). `pill` pour les cartes ; `line` pour la ligne d'état au-dessus de la
 * fiche dossier, suivie de l'identifiant du dossier. Un dossier sans état
 * (RM-04) n'en reçoit aucune : l'appelant n'appelle pas ce composant.
 */
export default function LawStateBadge({
  kind,
  variant = 'pill',
  uid,
  className = '',
}: {
  kind: LawStateKind
  variant?: 'pill' | 'line'
  uid?: string
  className?: string
}) {
  if (variant === 'line') {
    return (
      <p
        className={`flex items-center gap-1.5 text-[11px] font-semibold tracking-[0.065em] text-ink-faint uppercase ${className}`}
      >
        <LawStateIcon kind={kind} />
        {LAW_STATE_LABELS[kind]}
        {uid && ` · ${uid}`}
      </p>
    )
  }
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-[3px] text-[12.5px] leading-[18px] font-semibold ${LAW_STATE_TONES[kind]} ${className}`}
    >
      <LawStateIcon kind={kind} />
      {LAW_STATE_LABELS[kind]}
    </span>
  )
}
