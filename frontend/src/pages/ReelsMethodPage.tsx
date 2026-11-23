import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { ErrorPanel, Loading, PageHeader } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import type { ReelsMethodResponse } from '../types/reel'

/**
 * Méthode publique des Reels de votes (RM-15, RM-16) : planning, critères et
 * rotation des thèmes sont lus de l'API, qui les tire des règles qu'elle applique.
 */
export default function ReelsMethodPage() {
  const { data, error, isPending } = useQuery<ReelsMethodResponse>({
    queryKey: ['reels-methode'],
    queryFn: () => fetchJson<ReelsMethodResponse>('/api/reels/methode'),
  })

  if (isPending) return <Loading>Chargement de la méthode…</Loading>
  if (error) return <ErrorPanel error={error} />

  return (
    <div className="space-y-9">
      <PageHeader
        title="Comment sont préparés les Reels de votes ?"
        lede={`Un Reel est préparé chaque jour, prêt à ${data.due_time}. Cette page décrit le planning, les critères et la rotation des thèmes appliqués.`}
      />

      <section className="border-t border-line pt-5">
        <h3 className="heading-5 text-ink">Le planning de la semaine</h3>
        <ul className="mt-3 divide-y divide-line border-y border-line text-[15px] text-ink-soft lg:max-w-3xl">
          {data.days.map((day) => (
            <li key={day.weekday} className="py-2.5">
              <span className="font-semibold capitalize text-ink">{day.weekday}</span> : {day.period_label}
            </li>
          ))}
        </ul>
      </section>

      <div className="grid gap-x-14 gap-y-9 lg:grid-cols-2">
        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">Le vote le plus suivi</h3>
          <p className="mt-2 text-[15px] leading-[1.55] text-ink-soft">{data.most_followed_rule}</p>
        </section>
        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">Quand un Reel est publiable</h3>
          <p className="mt-2 text-[15px] leading-[1.55] text-ink-soft">{data.completeness_rule}</p>
        </section>
        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">La priorité du lundi</h3>
          <p className="mt-2 text-[15px] leading-[1.55] text-ink-soft">{data.monday_priority}</p>
        </section>
        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">La rotation des thèmes</h3>
          <ol className="mt-2 list-decimal space-y-0.5 pl-5 text-[15px] text-ink-soft">
            {data.theme_rotation.map((code) => (
              <li key={code}>{code.replaceAll('-', ' ')}</li>
            ))}
          </ol>
        </section>
      </div>

      <p className="text-[15px] text-ink-soft">
        <Link to="/semaine" className="link">
          Voir la semaine de votes
        </Link>
      </p>
    </div>
  )
}
