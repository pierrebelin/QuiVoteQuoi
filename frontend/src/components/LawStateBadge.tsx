import { LAW_STATE_DOTS, LAW_STATE_LABELS, LAW_STATE_WORDS } from '../lib/lawState'
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

/** Pastille ronde d'un état (refonte A) : 18 px, l'icône de l'état dedans. */
function LawStateDot({ kind }: { kind: LawStateKind }) {
  return (
    <span
      aria-hidden
      className={`inline-flex size-[18px] shrink-0 items-center justify-center rounded-full ${LAW_STATE_DOTS[kind]}`}
    >
      <LawStateIcon kind={kind} className="size-3" />
    </span>
  )
}

/**
 * État d'un dossier (En navette, Adopté définitivement, Devenu loi), en
 * pastille ronde et mot (refonte A). `pill` pour les cartes, suivie du
 * complément quand il y en a un (`lawStateDetail`) ; `line` pour la ligne d'état
 * au-dessus de la fiche dossier, suivie de l'identifiant du dossier. Un dossier
 * sans état (RM-04) n'en reçoit aucune : l'appelant n'appelle pas ce composant.
 */
export default function LawStateBadge({
  kind,
  variant = 'pill',
  uid,
  detail,
  className = '',
}: {
  kind: LawStateKind
  variant?: 'pill' | 'line'
  uid?: string
  /** Pastille seulement : « modifié par le Sénat le 20 nov. », « promulgation attendue »… */
  detail?: string | null
  className?: string
}) {
  if (variant === 'line') {
    return (
      <p
        className={`flex flex-wrap items-center gap-x-1.5 gap-y-1 text-[13px] leading-[18px] font-semibold ${LAW_STATE_WORDS[kind]} ${className}`}
      >
        <LawStateDot kind={kind} />
        <span>{LAW_STATE_LABELS[kind]}</span>
        {uid && <span className="font-medium text-ink-faint tabular-nums">· {uid}</span>}
      </p>
    )
  }
  return (
    <span
      className={`inline-flex items-center gap-1.5 text-[13px] leading-[18px] font-semibold ${LAW_STATE_WORDS[kind]} ${className}`}
    >
      <LawStateDot kind={kind} />
      <span>
        {LAW_STATE_LABELS[kind]}
        {detail && <span className="font-medium text-ink">{` · ${detail}`}</span>}
      </span>
    </span>
  )
}
