import { Link } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchJson } from '../lib/fetchJson'
import type { FreshnessResponse, SourceFreshness } from '../types/freshness'

const REFRESH_MS = 10 * 60 * 1000

const dateTime = new Intl.DateTimeFormat('fr-FR', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})
const dateOnly = new Intl.DateTimeFormat('fr-FR', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
})

export function useFreshness() {
  return useQuery<FreshnessResponse>({
    queryKey: ['freshness'],
    queryFn: () => fetchJson<FreshnessResponse>('/api/freshness'),
    refetchInterval: REFRESH_MS,
  })
}

const formatDateTime = (iso: string | null) =>
  iso ? dateTime.format(new Date(iso)) : 'jamais'

const lateSources = (data: FreshnessResponse): SourceFreshness[] =>
  data.sources.filter((source) => source.status !== 'up_to_date')

/** Ligne de pied de page : à jour, en retard (source et dernière réussite), ou indisponible sans rien supposer. */
export function FreshnessFooterLine() {
  const { data, isError } = useFreshness()
  if (isError || !data) {
    return (
      <span className="text-ink-faint">
        Date de dernière vérification indisponible
      </span>
    )
  }
  const late = lateSources(data)
  const lastScrutin = data.last_scrutin_date
    ? `dernier vote publié le ${dateOnly.format(new Date(data.last_scrutin_date))}`
    : ''
  if (late.length === 0) {
    return (
      <span className="text-ink-soft">
        {['Données synchronisées', ...(lastScrutin ? [lastScrutin] : [])].join(' · ')} ·{' '}
        <Link to="/comprendre#fraicheur" className="link">
          détail
        </Link>
      </span>
    )
  }
  const delayed = late.filter((source) => source.last_success_at)
  const never = late.filter((source) => !source.last_success_at)
  const parts = [
    ...delayed.map(
      (source) => `${source.label} : dernière réussite le ${formatDateTime(source.last_success_at)}`,
    ),
    ...(delayed.length > 0 ? ['les éléments votés depuis ne sont pas encore affichés'] : []),
    ...(never.length > 0
      ? [
          `${never.length === 1 ? 'Jamais synchronisée' : 'Jamais synchronisées'} : ${never
            .map((source) => source.label)
            .join(', ')}`,
        ]
      : []),
    ...(lastScrutin ? [lastScrutin] : []),
  ]
  return (
    <span role="status" className="text-info">
      {parts.join(' · ')} ·{' '}
      <Link to="/comprendre#fraicheur" className="link">
        détail
      </Link>
    </span>
  )
}

function attemptText(source: SourceFreshness): string {
  if (!source.last_attempt_at) return 'aucune tentative'
  const outcome = source.last_attempt_succeeded ? 'réussie' : 'échouée'
  const streak =
    source.consecutive_failures > 1
      ? ` (${source.consecutive_failures} de suite)`
      : ''
  return `${formatDateTime(source.last_attempt_at)} · ${outcome}${streak}`
}

/** Section « Fraîcheur des données » de la page Comprendre. */
export function FreshnessSection() {
  const { data, isLoading, isError } = useFreshness()
  return (
    <section id="fraicheur" className="scroll-mt-6 space-y-3">
      <h2 className="heading-4">Fraîcheur des données</h2>
      <p className="text-sm text-ink-soft">
        Une source passe « en retard » après 6 h sans synchronisation réussie.
        Le motif d'un échec n'est pas publié.
      </p>
      {isLoading && <p className="text-sm text-ink-faint">Chargement…</p>}
      {isError && (
        <p className="text-sm text-ink-faint">
          L'état des sources est indisponible pour le moment.
        </p>
      )}
      {data && (
        <>
          <ul className="grid gap-2">
            {data.sources.map((source) => (
              <li
                key={source.source}
                className="grid gap-x-4 gap-y-1 rounded border border-line px-4 py-3 text-sm sm:grid-cols-[1.2fr_auto_1fr_1fr]"
              >
                <span className="font-semibold">{source.label}</span>
                <span className="font-semibold">{source.status_label}</span>
                <span>
                  <span className="block text-xs text-ink-faint">
                    Dernière réussite
                  </span>
                  {formatDateTime(source.last_success_at)}
                </span>
                <span>
                  <span className="block text-xs text-ink-faint">
                    Dernière tentative
                  </span>
                  {attemptText(source)}
                </span>
              </li>
            ))}
          </ul>
          <p className="text-sm text-ink-soft">
            Dernier vote publié :{' '}
            <strong>
              {data.last_scrutin_date
                ? dateOnly.format(new Date(data.last_scrutin_date))
                : 'aucun'}
            </strong>
          </p>
        </>
      )}
    </section>
  )
}
