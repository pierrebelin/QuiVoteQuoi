import type { ReactNode } from 'react'
import { Link } from 'react-router'
import { LawStateIcon } from './LawStateBadge'
import { LAW_STATE_TONES } from '../lib/lawState'
import type { LawStateKind } from '../types/dossiers'

/**
 * Titre d'un groupe de textes rangés par état (la semaine, l'accueil) : vert
 * pour les textes qui vont s'appliquer, gris pour la navette (RM-07).
 */
export function LawStateGroupHeading({
  kind,
  first = false,
  className = '',
  children,
}: {
  kind: LawStateKind
  /** Premier groupe : pas de marge au-dessus. */
  first?: boolean
  className?: string
  children: ReactNode
}) {
  return (
    <h3
      className={`mb-1 flex items-center gap-1.5 text-[13px] font-semibold ${
        first ? '' : 'mt-4'
      } ${kind === 'in_shuttle' ? 'text-ink-soft' : 'text-yes'} ${className}`}
    >
      <LawStateIcon kind={kind} />
      {children}
    </h3>
  )
}

/**
 * Ligne d'un texte rangé par état : pastille de l'état, titre, puis une ligne
 * de détail fournie par l'appelant. Mène à la fiche dossier.
 */
export function LawStateRow({
  kind,
  dossierUid,
  title,
  className = '',
  titleClassName = '',
  children,
}: {
  kind: LawStateKind
  dossierUid: string
  title: string
  /** Classes de la ligne (visibilité selon la largeur). */
  className?: string
  /** Classes du titre (coupe à N lignes selon la largeur). */
  titleClassName?: string
  children: ReactNode
}) {
  return (
    <Link
      to={`/dossiers/${dossierUid}`}
      className={`group flex items-start gap-3 border-t border-line py-[11px] text-ink hover:text-ink ${className}`}
    >
      <span className={`flex size-7 shrink-0 items-center justify-center rounded-full ${LAW_STATE_TONES[kind]}`}>
        <LawStateIcon kind={kind} className="size-3.5" />
      </span>
      <span className="min-w-0">
        <span className={`block text-[14.5px] leading-5 font-semibold group-hover:underline ${titleClassName}`}>
          {title}
        </span>
        <span className="mt-0.5 block text-[13px] leading-[19px] text-ink-soft">{children}</span>
      </span>
    </Link>
  )
}
