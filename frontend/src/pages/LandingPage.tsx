import { useQuery } from '@tanstack/react-query'
import { Link, Navigate, useLocation } from 'react-router'
import { useFreshness } from '../components/Freshness'
import VoteCard from '../components/VoteCard'
import { finalVoteCard } from '../components/voteCardData'
import { FOCUS_RING } from '../components/entry/styles'
import { Card, Loading } from '../components/ui'
import {
  ENTRY_PATH,
  PARAM_GROUPS,
  PARAM_STEP,
  PARAM_THEMES,
} from '../lib/entryFlow'
import { fetchJson } from '../lib/fetchJson'
import type { FinalVoteListResponse } from '../types/finalVotes'
import type { ScrutinListResponse } from '../types/scrutins'

/**
 * Usages du site, formulés comme des gestes de lecteur et non comme le nom des
 * tables. Aucun ne promet une conclusion : le site fournit la pièce, le
 * rapprochement appartient au lecteur (README.md §6).
 */
const USAGES = [
  {
    to: '/votes-par-groupe',
    titre: 'Confronter un discours à un vote',
    texte: 'La position de vote d’un groupe est datée et publique : pour, contre, abstention, vote par vote.',
  },
  {
    to: '/scrutins',
    titre: 'Voir où un groupe se divise',
    texte: 'La liste nominale montre qui a voté autrement que les siens, et sur quel texte.',
  },
  {
    to: ENTRY_PATH,
    titre: 'Suivre un sujet qui vous concerne',
    texte: 'Logement, salaires, santé, énergie : les textes du quotidien et les votes qu’ils ont produits.',
  },
  {
    to: '/dossiers',
    titre: 'Savoir d’où vient un texte',
    texte: 'Qui l’a déposé, son passage en commission puis en séance, ce qu’il est devenu.',
  },
]

/** Les engagements vérifiables, en trois lignes ; leur détail est sur /a-propos. */
const REGLES = [
  {
    titre: 'Tout est publié.',
    texte: 'Chaque vote de l’Assemblée, sans sélection ; une lacune connue est signalée.',
  },
  {
    titre: 'Des chiffres bruts.',
    texte: 'Pas d’adverbes, et la source officielle sur chaque page.',
  },
  {
    titre: 'Aucun jugement.',
    texte: 'Ni note, ni classement, ni score de groupe.',
  },
]

const shortDate = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' })

function Arrow({ className = 'size-4' }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={className}
    >
      <path d="M5 12h14M13 6l6 6-6 6" />
    </svg>
  )
}

function CheckBullet() {
  return (
    <span className="mt-0.5 inline-flex size-5 shrink-0 items-center justify-center rounded-full bg-accent-soft text-accent">
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={3}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden
        className="size-3"
      >
        <path d="M5 12.5l4.5 4.5L19 7.5" />
      </svg>
    </span>
  )
}

/**
 * Repères chiffrés, tous lus dans l'API : rien ne s'affiche tant qu'un chiffre
 * n'est pas arrivé, et « à jour » n'est écrit que si aucune source n'est en
 * retard.
 */
function Landmarks({ groupCount }: { groupCount: number | null }) {
  const total = useQuery({
    queryKey: ['scrutins', 'total'],
    queryFn: () => fetchJson<ScrutinListResponse>('/api/scrutins?limit=1&offset=0'),
    select: (data) => data.total,
    retry: false,
  })
  const freshness = useFreshness()
  const upToDate =
    freshness.data &&
    freshness.data.last_scrutin_date &&
    freshness.data.sources.every((source) => source.status === 'up_to_date')
      ? shortDate.format(new Date(freshness.data.last_scrutin_date))
      : null

  const chips = [
    total.data !== undefined && (
      <>
        <b className="font-semibold text-ink">{total.data.toLocaleString('fr-FR')}</b> votes
      </>
    ),
    groupCount !== null && groupCount > 0 && (
      <>
        <b className="font-semibold text-ink">{groupCount}</b> groupes
      </>
    ),
    upToDate && <>à jour au {upToDate}</>,
  ].filter(Boolean)

  if (chips.length === 0) return null
  return (
    <ul aria-label="Repères" className="mt-4 flex flex-wrap justify-center gap-1.5 md:justify-start">
      {chips.map((chip, index) => (
        <li
          key={index}
          className="rounded-full bg-surface-soft px-2.5 py-1 text-[12.5px] font-medium text-ink-soft tabular-nums"
        >
          {chip}
        </li>
      ))}
    </ul>
  )
}

/**
 * Accueil : ce que fait le site, en un écran, puis où aller. Le parcours par
 * sujet vit sur /par-sujet, la méthode et les sources sur /a-propos.
 *
 * Les anciennes adresses du parcours (`/?sujets=…`), partagées avant son
 * déménagement, y sont renvoyées telles quelles.
 */
export default function LandingPage() {
  const location = useLocation()
  const params = new URLSearchParams(location.search)
  const legacyEntry = [PARAM_THEMES, PARAM_GROUPS, PARAM_STEP].some((key) => params.has(key))

  // `tous_groupes` ramène la position de chaque groupe ; le référentiel des
  // groupes voyage avec la réponse.
  const latest = useQuery({
    queryKey: ['votes-finaux', 'dernier'],
    queryFn: () =>
      fetchJson<FinalVoteListResponse>('/api/votes-finaux?tous_groupes=true&limit=1&offset=0'),
    enabled: !legacyEntry,
    retry: false,
  })

  if (legacyEntry) return <Navigate to={`${ENTRY_PATH}${location.search}`} replace />

  const vote = latest.data?.items[0]
  const groups = latest.data?.groups ?? []

  return (
    <div className="mx-auto max-w-5xl space-y-8 md:space-y-12">
      <div className="grid items-start gap-8 md:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)] md:gap-10">
        <section aria-labelledby="promesse" className="md:pt-6">
          <p className="text-xs font-semibold tracking-[0.06em] text-accent uppercase">
            Assemblée nationale
          </p>
          <h2
            id="promesse"
            className="mt-2 text-[30px] leading-[1.12] font-semibold tracking-tight sm:text-5xl"
          >
            Les votes de l’Assemblée, sur pièces.
          </h2>
          <p className="mt-3 max-w-xl text-[15px] leading-relaxed text-ink-soft sm:text-base">
            Ce que chaque groupe a voté, texte par texte. Des chiffres bruts et la source
            officielle, sans sélection ni commentaire.
          </p>
          <div className="mt-5 flex flex-col gap-2 sm:flex-row sm:flex-wrap">
            <Link
              to={ENTRY_PATH}
              className={`inline-flex min-h-[50px] items-center justify-center gap-1.5 rounded-xl bg-accent px-5 text-[15px] font-semibold text-surface transition-colors hover:bg-accent-strong ${FOCUS_RING}`}
            >
              Voir les votes par sujet
              <Arrow />
            </Link>
            <Link
              to="/scrutins"
              className={`inline-flex min-h-12 items-center justify-center rounded-xl border-[1.5px] border-line bg-surface px-5 text-[15px] font-semibold text-ink transition-colors hover:bg-surface-soft ${FOCUS_RING}`}
            >
              Derniers votes
            </Link>
          </div>
          <Landmarks groupCount={latest.data ? latest.data.groups.length : null} />
        </section>

        <section aria-labelledby="dernier-vote" className="space-y-2.5">
          <h3 id="dernier-vote" className="text-lg font-semibold tracking-tight">
            Dernier vote sur l’ensemble d’un texte
          </h3>
          {vote ? (
            <VoteCard size="full" {...finalVoteCard(vote, groups)} />
          ) : (
            <Card className="px-4 py-6">
              {latest.isError ? (
                <div className="space-y-2">
                  <p className="text-sm font-semibold text-ink">
                    Le dernier vote n’est pas disponible pour le moment.
                  </p>
                  <p className="text-sm leading-relaxed text-ink-soft">
                    Aucun résultat n’est remplacé par une estimation.
                  </p>
                  <button
                    type="button"
                    onClick={() => void latest.refetch()}
                    className="text-sm font-medium text-accent hover:underline"
                  >
                    Réessayer
                  </button>
                </div>
              ) : latest.data ? (
                <p className="text-sm text-ink-soft">Aucun vote sur l’ensemble n’est encore publié.</p>
              ) : (
                <Loading>Chargement du dernier vote…</Loading>
              )}
            </Card>
          )}
        </section>
      </div>

      <div className="grid items-start gap-8 md:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)] md:gap-10">
        <section aria-labelledby="usages">
          <h3 id="usages" className="text-lg font-semibold tracking-tight">
            À quoi ça sert
          </h3>
          <ul className="mt-2">
            {USAGES.map((usage) => (
              <li key={usage.titre}>
                <Link
                  to={usage.to}
                  className={`group flex items-center gap-3 border-t border-line py-3 ${FOCUS_RING}`}
                >
                  <span className="min-w-0 flex-1">
                    <span className="block text-[14.5px] font-semibold text-ink transition-colors group-hover:text-accent">
                      {usage.titre}
                    </span>
                    <span className="mt-0.5 block text-[13px] leading-snug text-ink-soft">
                      {usage.texte}
                    </span>
                  </span>
                  <Arrow className="size-4 shrink-0 text-ink-faint transition-colors group-hover:text-accent" />
                </Link>
              </li>
            ))}
          </ul>
        </section>

        <section
          aria-labelledby="regles"
          className="rounded-2xl border border-line bg-surface p-4 shadow-card sm:p-5"
        >
          <h3 id="regles" className="text-[15px] font-semibold">
            Trois règles
          </h3>
          <ul className="mt-3 space-y-2.5">
            {REGLES.map((regle) => (
              <li key={regle.titre} className="flex items-start gap-2.5">
                <CheckBullet />
                <span className="text-[13.5px] leading-snug text-ink-soft">
                  <b className="font-semibold text-ink">{regle.titre}</b> {regle.texte}
                </span>
              </li>
            ))}
          </ul>
          <Link
            to="/a-propos"
            className={`mt-3.5 flex min-h-10 items-center justify-between border-t border-line pt-1 text-sm font-medium text-accent hover:underline ${FOCUS_RING}`}
          >
            Méthode, sources et qui fait ce site
            <Arrow />
          </Link>
        </section>
      </div>
    </div>
  )
}
