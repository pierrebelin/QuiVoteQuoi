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

/** Bandeau sous l'en-tête, tant qu'une source est en retard. Rien si l'état est inconnu. */
export function FreshnessBanner() {
  const { data } = useFreshness()
  if (!data) return null
  const late = lateSources(data)
  if (late.length === 0) return null

  return (
    <div
      role="status"
      className="border-b border-line bg-info-soft px-6 py-3 text-sm text-info"
    >
      <p className="mx-auto max-w-7xl">
        {late.map((source) => (
          <span key={source.source}>
            <strong>
              {source.label} : dernière synchronisation réussie{' '}
              {source.last_success_at
                ? `le ${formatDateTime(source.last_success_at)}`
                : 'inexistante'}
              .
            </strong>{' '}
          </span>
        ))}
        Les éléments votés depuis ne sont pas encore affichés.{' '}
        <Link to="/comprendre#fraicheur" className="font-semibold underline">
          Détail
        </Link>
      </p>
    </div>
  )
}

/** Ligne de pied de page : à jour, en retard, ou indisponible sans rien supposer. */
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
    ? ` · dernier scrutin publié le ${dateOnly.format(new Date(data.last_scrutin_date))}`
    : ''
  return (
    <span className="text-ink-soft">
      {late.length === 0
        ? `Données synchronisées${lastScrutin}`
        : `${late.map((s) => s.label).join(', ')} : synchronisation en retard${lastScrutin}`}{' '}
      ·{' '}
      <Link to="/comprendre#fraicheur" className="text-accent">
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
      <h2 className="text-lg font-semibold">Fraîcheur des données</h2>
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
                className="grid gap-x-4 gap-y-1 rounded-xl border border-line px-4 py-3 text-sm sm:grid-cols-[1.2fr_auto_1fr_1fr]"
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
            Dernier scrutin publié :{' '}
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
