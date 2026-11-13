import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import VotesTabs from '../components/VotesTabs'
import DayPills from '../components/week/DayPills'
import WeekTextCard from '../components/week/WeekTextCard'
import { CoverageNote } from '../components/ScrutinList'
import { ErrorPanel, Loading } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import type { WeekResponse } from '../types/week'

/**
 * La semaine de votes : pastilles de jours, puis une carte par texte, les
 * textes sans dossier en dernier. Navigation entre semaines : autre lot.
 */
export default function WeekPage() {
  const { date } = useParams()
  const [day, setDay] = useState<string | null>(null)
  const { data, error, isPending } = useQuery({
    queryKey: ['week', date ?? 'current'],
    queryFn: () => fetchJson<WeekResponse>(date ? `/api/semaine/${date}` : '/api/semaine'),
  })

  if (isPending) return <Loading>Chargement de la semaine…</Loading>
  if (error) return <ErrorPanel error={error} />

  const today = new Date().toISOString().slice(0, 10)
  return (
    <div className="space-y-4">
      <VotesTabs current="semaine" />
      <nav aria-label="Changer de semaine" className="flex items-center justify-between gap-2 text-sm">
        {data.previous ? (
          <Link to={`/semaine/${data.previous.monday}`} onClick={() => setDay(null)} className="font-medium text-accent">
            ← Semaine précédente ({data.previous.votes} vote{data.previous.votes > 1 ? 's' : ''})
          </Link>
        ) : (
          <span />
        )}
        {data.next && (
          <Link to={`/semaine/${data.next}`} onClick={() => setDay(null)} className="font-medium text-accent">
            Semaine suivante →
          </Link>
        )}
      </nav>
      <DayPills days={data.days} selected={day} onSelect={setDay} />
      <div className="space-y-4">
        {data.texts.map((text) => (
          <WeekTextCard key={text.dossier_uid ?? 'sans-dossier'} text={text} day={day} today={today} />
        ))}
      </div>
      <CoverageNote note={data.coverage_note} />
    </div>
  )
}
