import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { Card, ErrorPanel, Loading, PageHeader } from '../components/ui'
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
    <div className="space-y-5">
      <PageHeader
        title="Comment sont préparés les Reels de votes ?"
        lede={`Un Reel est préparé chaque jour, prêt à ${data.due_time}. Cette page décrit le planning, les critères et la rotation des thèmes appliqués.`}
      />

      <Card className="px-4 py-3.5">
        <h3 className="text-sm font-semibold">Le planning de la semaine</h3>
        <ul className="mt-2 space-y-1 text-sm text-ink-soft">
          {data.days.map((day) => (
            <li key={day.weekday}>
              <span className="font-semibold capitalize text-ink">{day.weekday}</span> : {day.period_label}
            </li>
          ))}
        </ul>
      </Card>

      <div className="grid gap-3 lg:grid-cols-2">
        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">Le vote le plus suivi</h3>
          <p className="mt-2 text-sm leading-relaxed text-ink-soft">{data.most_followed_rule}</p>
        </Card>
        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">Quand un Reel est publiable</h3>
          <p className="mt-2 text-sm leading-relaxed text-ink-soft">{data.completeness_rule}</p>
        </Card>
        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">La priorité du lundi</h3>
          <p className="mt-2 text-sm leading-relaxed text-ink-soft">{data.monday_priority}</p>
        </Card>
        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">La rotation des thèmes</h3>
          <ol className="mt-2 list-decimal space-y-0.5 pl-5 text-sm text-ink-soft">
            {data.theme_rotation.map((code) => (
              <li key={code}>{code.replaceAll('-', ' ')}</li>
            ))}
          </ol>
        </Card>
      </div>

      <p className="text-sm text-ink-soft">
        <Link to="/semaine" className="link">
          Voir la semaine de votes
        </Link>
      </p>
    </div>
  )
}
