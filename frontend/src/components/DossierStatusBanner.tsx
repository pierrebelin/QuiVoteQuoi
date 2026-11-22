import LawStateBadge from './LawStateBadge'
import { dossierBanner, formatShortDate } from '../lib/dossierStatus'
import { lawStateDetail } from '../lib/lawState'
import type { DossierStatusDto } from '../types/dossiers'

/**
 * Bandeau du haut d'une carte de dossier : à gauche le sort ou l'étape, à
 * droite le dernier acte daté. Un acte daté après aujourd'hui est annoncé à
 * venir. Partagé par la liste des dossiers et celle des scrutins, pour que le
 * même dossier se lise pareil sur les deux pages.
 */
export default function DossierStatusBanner({
  dossier,
  today,
  withLawState = false,
}: {
  dossier: DossierStatusDto
  /** Date du jour (AAAA-MM-JJ). */
  today: string
  /**
   * Étiquette « où en est la loi » à la place du sort (liste par texte) ; un
   * dossier sans état garde le bandeau (RM-04).
   */
  withLawState?: boolean
}) {
  const state = dossierBanner(dossier)
  const upcoming = dossier.last_activity_date > today
  const step = dossier.last_activity_label.replace(/\.$/, '')
  const lawState = withLawState ? dossier.law_state : null

  const lastStep = (
    <span className="text-[12.5px] font-medium text-ink-soft">
      {upcoming ? 'Prochaine étape' : 'Dernière étape'} : {step} · {formatShortDate(dossier.last_activity_date, today)}
      {upcoming && (
        <span className="ml-1.5 rounded bg-surface-soft px-1.5 text-[11px] font-semibold text-info">à venir</span>
      )}
    </span>
  )

  // Une loi publiée n'a plus d'étape : la promulgation, déjà dans
  // l'étiquette, serait dite deux fois. Sous 768 px, l'étiquette complète
  // (jusqu'à 330 px) ne tient pas dans la carte : la pastille ne porte que
  // l'état, le complément la suit en texte et passe à la ligne.
  if (lawState) {
    const detail = lawStateDetail(lawState, today)
    return (
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1.5 px-[18px] pt-3.5">
        <LawStateBadge kind={lawState.kind} detail={detail} className="max-md:hidden" />
        <p className="text-[13px] leading-[24px] font-medium text-ink md:hidden">
          <LawStateBadge kind={lawState.kind} className="mr-2 align-top" />
          {detail}
        </p>
        {lawState.kind !== 'became_law' && lastStep}
      </div>
    )
  }

  return (
    <div
      className={`flex flex-wrap items-center justify-between gap-x-4 gap-y-0.5 px-[18px] py-1.5 text-[13px] font-semibold ${state.tone}`}
    >
      <span>{state.text}</span>
      <span className="text-[12.5px] font-medium text-ink-soft">
        {upcoming ? 'Prochaine étape' : 'Dernière étape'} : {step} ·{' '}
        {formatShortDate(dossier.last_activity_date, today)}
        {upcoming && (
          <span className="ml-1.5 rounded bg-surface px-1.5 text-[11px] font-semibold text-info">
            à venir
          </span>
        )}
      </span>
    </div>
  )
}
