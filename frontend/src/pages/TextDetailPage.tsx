import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { BackLink } from '../components/ui'
import { FamilyBadges, UnassignedNote } from '../components/ThemeBadges'
import type { TextDetailResponse } from '../types/themes'

function frenchDate(value: string) {
  return new Date(value).toLocaleDateString('fr-FR')
}

export default function TextDetailPage() {
  const { key } = useParams<{ key: string }>()

  const text = useQuery({
    queryKey: ['texts', key],
    queryFn: (): Promise<TextDetailResponse> =>
      fetch(`/api/texts/${encodeURIComponent(key!)}`).then((res) => {
        if (res.status === 404) throw new Error('Texte inconnu')
        if (!res.ok) throw new Error('Chargement impossible')
        return res.json()
      }),
  })

  if (text.isLoading)
    return <p className="animate-pulse text-ink-faint">Chargement…</p>
  if (text.error)
    return <p className="text-no">{(text.error as Error).message}</p>

  const data = text.data!

  return (
    <div className="space-y-3">
      <div>
        <BackLink to="/themes">Tous les thèmes</BackLink>
        <h2 className="mt-1 text-2xl font-semibold tracking-tight">
          {data.label}
        </h2>
        <p className="mt-0.5 text-xs text-ink-faint">
          {data.scrutin_count} scrutin{data.scrutin_count > 1 ? 's' : ''}
          {data.first_vote && data.last_vote
            ? ` · du ${frenchDate(data.first_vote)} au ${frenchDate(data.last_vote)}`
            : ''}
          {data.dossier_uid && (
            <>
              {' · dossier : '}
              <Link
                to={`/dossiers/${data.dossier_uid}`}
                className="text-accent underline"
              >
                {data.dossier_label ?? data.dossier_uid}
              </Link>
            </>
          )}
        </p>
      </div>

      <section>
        <h3 className="mb-1.5 text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
          Familles
        </h3>
        {data.families.length > 0 ? (
          <FamilyBadges families={data.families} />
        ) : (
          <UnassignedNote outcome={data.last_attempt_outcome} />
        )}
        {data.families.some((f) => f.motive) && (
          <ul className="mt-2 space-y-0.5 text-xs text-ink-soft">
            {data.families
              .filter((f) => f.motive)
              .map((f) => (
                <li key={f.code}>
                  <span className="font-medium text-ink">{f.label}</span> —{' '}
                  {f.motive}
                </li>
              ))}
          </ul>
        )}
      </section>

      <section>
        <h3 className="mb-1.5 text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
          Scrutins de ce texte
        </h3>
        {data.scrutins.length === 0 ? (
          <p className="text-sm text-ink-faint">Aucun scrutin listé.</p>
        ) : (
          <ul className="divide-y divide-line rounded-lg border border-line bg-surface">
            {data.scrutins.map((scrutin) => (
              <li
                key={scrutin.uid}
                className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-0.5 px-4 py-2"
              >
                <Link
                  to={`/scrutins/${scrutin.uid}`}
                  className="min-w-[min(100%,16rem)] flex-1 text-sm hover:text-accent hover:underline"
                >
                  {scrutin.subject}
                </Link>
                <p className="max-w-full shrink-0 text-xs text-ink-faint">
                  {frenchDate(scrutin.date)} · n° {scrutin.number} ·{' '}
                  {scrutin.outcome_label} · pour {scrutin.votes_for}, contre{' '}
                  {scrutin.votes_against}, abstention {scrutin.abstentions}
                </p>
              </li>
            ))}
          </ul>
        )}
        {data.scrutin_count > data.scrutins.length && (
          <p className="mt-1.5 text-xs text-ink-faint">
            {data.scrutins.length} scrutins affichés sur {data.scrutin_count}.
          </p>
        )}
      </section>
    </div>
  )
}
