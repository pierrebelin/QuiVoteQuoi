import { useQuery } from '@tanstack/react-query'
import { Link, Navigate, useLocation } from 'react-router'
import { useFreshness } from '../components/Freshness'
import VoteCard from '../components/VoteCard'
import { finalVoteCard } from '../components/voteCardData'
import { FOCUS_RING } from '../components/entry/styles'
import { InfoTip } from '../components/InfoTip'
import { Card, Loading, Note } from '../components/ui'
import { ALL_LOGOS } from '../content/logos'
import { PORTRAITS } from '../content/portraits'
import { GITHUB_REPOSITORY_URL } from '../lib/repository'
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
    to: '/scrutins?vue=texte',
    titre: 'Savoir d’où vient un texte',
    texte: 'Qui l’a déposé, son passage en commission puis en séance, ce qu’il est devenu.',
  },
]

/**
 * Ce qui tient la neutralité : trois contraintes vérifiables. La troisième est
 * le seul endroit du produit où un jugement entre (README.md §5) ; elle le dit
 * et renvoie à sa méthode.
 */
const REGLES = [
  {
    titre: 'Tout est publié.',
    texte: 'Chaque vote de l’Assemblée, sans sélection ; le tri ordonne, il ne filtre jamais, et une lacune connue est signalée.',
  },
  {
    titre: 'Des chiffres bruts.',
    texte: 'Pas d’adverbes, la position de chaque député au détail du vote, la source officielle sur chaque page.',
  },
  {
    titre: 'Un seul arbitrage, publié.',
    texte: 'Le rattachement des textes à des sujets : seul jugement du site, révisable et historisé.',
    lien: { to: '/themes/methode', label: 'La méthode' },
  },
]

/**
 * La contrepartie qui rend la promesse vérifiable : chaque ligne est une
 * fonctionnalité écartée, même facile (README.md §6, non-objectifs).
 */
const REFUS = [
  'Aucune note, aucun classement, aucun score de groupe ou de personne.',
  'Aucun score de cohérence : un écart entre un discours et un vote se constate, il ne se calcule pas.',
  'Aucune évaluation d’un texte : ni bon, ni mauvais, ni efficace.',
  'Aucun groupe parlementaire traduit en parti politique.',
  'Aucun chiffre produit par un modèle de langage.',
  'Aucune prédiction électorale ; les programmes 2027 sont posés côte à côte, sans verdict.',
]

/**
 * Les pièces qui documentent un vote. Le site ne rédige pas les motivations des
 * députés : il rassemble ce qui est attribué et publié, le lecteur conclut
 * (README.md §6).
 */
const PIECES = [
  {
    titre: 'Ce qui a été mis aux voix',
    texte: 'Un amendement, un article, une motion ou le texte entier : l’intitulé officiel, affiché en entier.',
  },
  {
    titre: 'Le texte',
    texte: 'Son auteur et son exposé des motifs, son passage en commission puis en séance, son sort.',
  },
  {
    titre: 'Chaque député',
    texte: 'Nom par nom : les divisions qu’un total de groupe efface, la consigne distinguée du vote réel.',
  },
  {
    titre: 'Les mises au point',
    texte: 'Un député déclare après coup avoir voulu voter autrement : affiché à part, sans changer le décompte.',
  },
]

const NEVER_SHOWN =
  'Jamais affiché : une motivation reconstituée, un commentaire de presse, une synthèse rédigée par un modèle de langage.'

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
 * Titre de section de l'accueil : surtitre en accent, titre, et sur téléphone
 * le chapô replié dans une infobulle à côté du titre (sous 768 px, les textes
 * explicatifs passent en infobulle).
 */
function SectionHead({
  id,
  surtitre,
  titre,
  chapo,
  info,
}: {
  id: string
  surtitre: string
  titre: string
  chapo?: string
  /** Mention que le téléphone ne montre que dans l'infobulle. */
  info?: string
}) {
  return (
    <>
      <p className="text-xs font-semibold tracking-[0.06em] text-accent uppercase">{surtitre}</p>
      <h3 id={id} className="mt-1.5 heading-3 sm:heading-2">
        {titre}
        {(chapo || info) && (
          <InfoTip label={`En savoir plus : ${surtitre}`} className="md:hidden">
            {chapo && <p>{chapo}</p>}
            {info && <p className="mt-2">{info}</p>}
          </InfoTip>
        )}
      </h3>
      {chapo && (
        <p className="mt-2 hidden max-w-3xl text-[15px] leading-relaxed text-ink-soft md:block">
          {chapo}
        </p>
      )}
    </>
  )
}

/** Crédits des logos et des portraits : Wikimedia Commons, dus par leur licence,
 * ou photos de campagne reprises de leur source. */
function Credits() {
  const credits = [
    ...ALL_LOGOS.map((logo) => logo.credit),
    ...Object.values(PORTRAITS).map((portrait) => portrait.credit),
  ]
  return (
    <details className="mt-4 border-t border-line pt-3 text-sm text-ink-soft">
      <summary className="cursor-pointer font-medium text-ink">
        Crédits des logos et des portraits
      </summary>
      <p className="mt-2 leading-relaxed">
        Logos des groupes et des partis, portraits des candidats : Wikimedia Commons, sous la
        licence indiquée. À défaut de portrait libre, la photo publiée par la campagne est
        reprise de la source citée. Les portraits sont réduits, réencodés et, quand on la
        connaît, datés de leur prise de vue.
        Un logo identifie une organisation et ne l’engage en rien envers ce site.
      </p>
      <ul className="mt-2 space-y-1 text-xs">
        {credits.map((credit, index) => (
          <li key={`${index}-${credit.file}`}>
            <a href={credit.page} target="_blank" rel="noreferrer" className="link">
              {credit.file}
            </a>{' '}
            · {credit.author} · {credit.license}
          </li>
        ))}
      </ul>
    </details>
  )
}

const EXTERNAL_LINK = 'text-sm font-medium link'

/**
 * Accueil : ce que fait le site, en un écran, puis où aller ; ensuite ce qu'on
 * trouve derrière un vote, les règles qui tiennent la neutralité, les sources.
 * Le parcours par sujet vit sur /par-sujet.
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
            className="mt-2 heading-1 sm:text-[58px] sm:leading-[58px]"
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
              className={`inline-flex min-h-[50px] items-center justify-center gap-1.5 rounded bg-accent px-5 text-[15px] font-semibold text-surface transition-colors hover:bg-accent-strong ${FOCUS_RING}`}
            >
              Voir les votes par sujet
              <Arrow />
            </Link>
            <Link
              to="/scrutins"
              className={`inline-flex min-h-12 items-center justify-center rounded border-[1.5px] border-line bg-surface px-5 text-[15px] font-semibold text-ink transition-colors hover:bg-surface-soft ${FOCUS_RING}`}
            >
              Derniers votes
            </Link>
            <Link
              to="/candidats-2027"
              className={`inline-flex min-h-12 items-center justify-center gap-2 rounded bg-accent-soft px-5 text-[15px] font-semibold text-accent transition-colors hover:bg-accent-soft/70 sm:basis-full lg:basis-auto ${FOCUS_RING}`}
            >
              Comparer les programmes de 2027
              <Arrow />
            </Link>
          </div>
          <Landmarks groupCount={latest.data ? latest.data.groups.length : null} />
        </section>

        <section aria-labelledby="dernier-vote" className="space-y-2.5">
          <h3 id="dernier-vote" className="heading-4">
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
                    className="text-sm font-medium link"
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
          <h3 id="usages" className="heading-4">
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
          className="rounded border border-line bg-surface p-4 shadow-card sm:p-5"
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
                  {regle.lien && (
                    <>
                      {' '}
                      <Link to={regle.lien.to} className="font-medium link">
                        {regle.lien.label}
                      </Link>
                    </>
                  )}
                </span>
              </li>
            ))}
          </ul>
          <h4 className="mt-4 border-t border-line pt-3.5 text-[13.5px] font-semibold text-ink">
            Ce que ce site ne fait pas
          </h4>
          <ul className="mt-2 space-y-1.5">
            {REFUS.map((refus) => (
              <li key={refus} className="flex items-start gap-2 text-[13px] leading-snug text-ink-soft">
                <span aria-hidden className="mt-[7px] h-px w-2.5 shrink-0 bg-ink-faint" />
                {refus}
              </li>
            ))}
          </ul>
        </section>
      </div>

      <section aria-labelledby="pieces" className="border-t border-line pt-8">
        <SectionHead
          id="pieces"
          surtitre="Derrière chaque vote"
          titre="Un décompte ne dit pas les raisons. Les pièces, si."
          chapo="Le site ne reconstitue pas les motivations d’un groupe et ne les résume pas à sa place : il rassemble ce qui est attribué, daté et public autour de chaque vote."
          info={NEVER_SHOWN}
        />
        <ol className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {PIECES.map((piece, index) => (
            <li key={piece.titre}>
              <Card className="h-full px-4 py-3.5">
                <h4 className="flex items-center gap-2 text-sm font-semibold">
                  <span className="inline-flex size-6 shrink-0 items-center justify-center rounded bg-accent-soft text-[11px] font-semibold text-accent tabular-nums">
                    {index + 1}
                  </span>
                  {piece.titre}
                </h4>
                <p className="mt-1 text-[13px] leading-snug text-ink-soft">{piece.texte}</p>
              </Card>
            </li>
          ))}
        </ol>
        <div className="mt-3">
          <Note desktopOnly>{NEVER_SHOWN}</Note>
        </div>
      </section>

      <section aria-labelledby="sources" className="border-t border-line pt-8">
        <SectionHead id="sources" surtitre="Sources" titre="Tout vient de l’Assemblée, et se vérifie" />
        <Card className="mt-5 px-4 py-4 sm:px-5">
          <div className="grid gap-5 md:grid-cols-3">
            <div>
              <h4 className="text-sm font-semibold">Données</h4>
              <p className="mt-1 text-[13.5px] leading-relaxed text-ink-soft">
                Open data de l’Assemblée nationale, sous Licence Ouverte : votes nominatifs,
                députés et groupes avec leurs appartenances datées, textes. Rien n’est saisi à
                la main ni corrigé en silence. Le Sénat est hors périmètre.
              </p>
              <a
                href="https://data.assemblee-nationale.fr"
                target="_blank"
                rel="noreferrer"
                className={`mt-1.5 inline-block ${EXTERNAL_LINK}`}
              >
                data.assemblee-nationale.fr ↗
              </a>
            </div>
            <div>
              <h4 className="text-sm font-semibold">Code source</h4>
              <p className="mt-1 text-[13.5px] leading-relaxed text-ink-soft">
                Ingestion, rattachement aux sujets, affichage : le dépôt public permet de tout
                vérifier, de signaler une erreur ou de proposer une correction.
              </p>
              <a
                href={GITHUB_REPOSITORY_URL}
                target="_blank"
                rel="noreferrer"
                className={`mt-1.5 inline-block ${EXTERNAL_LINK}`}
              >
                Le dépôt sur GitHub ↗
              </a>
            </div>
            <div>
              <h4 className="text-sm font-semibold">Vocabulaire</h4>
              <p className="mt-1 text-[13.5px] leading-relaxed text-ink-soft">
                Non-votant, abstention, scrutin solennel, motion de censure : chaque mot de la
                procédure est expliqué, en deux niveaux de lecture.
              </p>
              <Link to="/comprendre" className={`mt-1.5 inline-block ${EXTERNAL_LINK}`}>
                Comprendre
              </Link>
            </div>
          </div>
          <Credits />
        </Card>
      </section>
    </div>
  )
}
