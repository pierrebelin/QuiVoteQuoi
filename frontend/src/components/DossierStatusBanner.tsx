import { dossierBanner, formatShortDate, type DossierStatusLike } from '../lib/dossierStatus'

/**
 * Bandeau du haut d'une carte de dossier : à gauche le sort ou l'étape, à
 * droite le dernier acte daté. Un acte daté après aujourd'hui est annoncé à
 * venir. Partagé par la liste des dossiers et celle des scrutins, pour que le
 * même dossier se lise pareil sur les deux pages.
 */
export default function DossierStatusBanner({
  dossier,
  today,
}: {
  dossier: DossierStatusLike
  /** Date du jour (AAAA-MM-JJ). */
  today: string
}) {
  const state = dossierBanner(dossier)
  const upcoming = dossier.last_activity_date > today
  const step = dossier.last_activity_label.replace(/\.$/, '')

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
