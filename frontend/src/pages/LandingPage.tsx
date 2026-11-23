import type { CSSProperties } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, Navigate, useLocation } from 'react-router'
import BecomingLawCard from '../components/BecomingLawCard'
import CountUp from '../components/CountUp'
import { useFreshness } from '../components/Freshness'
import LatestVoteHemicycle from '../components/LatestVoteHemicycle'
import VoteCard, { VoteIssue } from '../components/VoteCard'
import { finalVoteCard } from '../components/voteCardData'
import { FOCUS_RING } from '../components/entry/styles'
import { InfoTip } from '../components/InfoTip'
import { Card, Loading, Note } from '../components/ui'
import DayBars from '../components/week/DayBars'
import WeekFigures from '../components/week/WeekFigures'
import { ALL_LOGOS } from '../content/logos'
import { PORTRAITS } from '../content/portraits'
import { GITHUB_REPOSITORY_URL } from '../lib/repository'
import { useReveal } from '../lib/useReveal'
import { votesWord, weekSpan } from '../lib/week'
import {
  ENTRY_PATH,
  PARAM_GROUPS,
  PARAM_STEP,
  PARAM_THEMES,
} from '../lib/entryFlow'
import { fetchJson } from '../lib/fetchJson'
import type { FinalVoteListResponse } from '../types/finalVotes'
import type { ScrutinListResponse } from '../types/scrutins'
import type { WeekResponse } from '../types/week'

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

/**
 * À partir de 1280 px, troisième carte à côté des règles et des refus : les
 * sources et les pièces d'un vote, qui ont chacune leur section sous 1280 px.
 */
const VERIFIABLE: { titre: string; texte: string; label: string; to: string; external?: boolean }[] = [
  {
    titre: 'Données',
    texte:
      'Open data de l’Assemblée nationale, sous Licence Ouverte. Rien n’est saisi à la main ni corrigé en silence. Le Sénat est hors périmètre.',
    label: 'data.assemblee-nationale.fr ↗',
    to: 'https://data.assemblee-nationale.fr',
    external: true,
  },
  {
    titre: 'Code source',
    texte: 'Le dépôt public permet de tout vérifier, de signaler une erreur ou de proposer une correction.',
    label: 'Le dépôt sur GitHub ↗',
    to: GITHUB_REPOSITORY_URL,
    external: true,
  },
  {
    titre: 'Derrière chaque vote',
    texte:
      'Ce qui a été mis aux voix, le texte, chaque député, les mises au point : jamais une motivation reconstituée.',
    label: 'Comprendre un vote',
    to: '/comprendre',
  },
]

const shortDate = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' })
const frenchNumber = (value: number) => value.toLocaleString('fr-FR')

/** Rang d'un élément dans sa cascade d'apparition (`.reveal-item`, index.css). */
const revealRank = (index: number) => ({ '--i': index }) as CSSProperties

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
      className={`${className} transition-transform motion-safe:group-hover:translate-x-0.5`}
    >
      <path d="M5 12h14M13 6l6 6-6 6" />
    </svg>
  )
}

/** Croix d'un refus : un trait de croix, dans le gris des mentions. */
function XMark() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className="mt-0.5 size-4 shrink-0 text-ink-faint"
    >
      <path d="M6 6l12 12M18 6L6 18" />
    </svg>
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

  const figures = [
    total.data !== undefined && (
      <>
        <b className="figure block text-[28px] leading-none tracking-normal text-ink">
          <CountUp value={total.data} format={frenchNumber} />
        </b>{' '}
        <span className="mt-1 block text-[13px] text-ink-faint">votes</span>
      </>
    ),
    groupCount !== null && groupCount > 0 && (
      <>
        <b className="figure block text-[28px] leading-none tracking-normal text-ink">
          <CountUp value={groupCount} />
        </b>{' '}
        <span className="mt-1 block text-[13px] text-ink-faint">groupes</span>
      </>
    ),
  ].filter(Boolean)

  if (figures.length === 0 && !upToDate) return null
  return (
    <ul
      aria-label="Repères"
      className="mt-6 grid grid-cols-2 border-b border-line md:max-w-md xl:col-start-2 xl:row-start-3 xl:mt-5 xl:max-w-none"
    >
      {figures.map((figure, index) => (
        <li
          key={index}
          className={`border-t border-line py-3.5 ${index > 0 ? 'border-l pl-[18px] xl:pl-5' : ''}`}
        >
          {figure}
        </li>
      ))}
      {upToDate && (
        <li
          key="a-jour"
          className="col-span-2 flex items-center gap-2 border-t border-line py-2.5 text-[13px] font-medium text-ink-soft"
        >
          <span aria-hidden className="live-dot" />à jour au {upToDate}
        </li>
      )}
    </ul>
  )
}

/**
 * Titre de section de l'accueil : surtitre en gris, titre, et sur téléphone
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
      <p className="text-[13px] font-semibold text-ink-faint">{surtitre}</p>
      <h3 id={id} className="mt-1.5 heading-4 sm:heading-2">
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
function Credits({ className = 'mt-4 border-t border-line pt-3' }: { className?: string }) {
  const credits = [
    ...ALL_LOGOS.map((logo) => logo.credit),
    ...Object.values(PORTRAITS).map((portrait) => portrait.credit),
  ]
  return (
    <details className={`text-sm text-ink-soft ${className}`}>
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
 * La semaine en cours, résumée : ses nombres et ses jours, sans sélection. Une
 * semaine en cours encore vide laisse la place à la dernière semaine votée,
 * comme sur la page La semaine ; le titre le dit. Sous 1280 px, une bande grise
 * bord à bord ; à partir de 1280 px, une colonne de 400 px dans la bande que
 * partage ce qui devient loi. Le bouton daté de l'accroche menant déjà à la
 * semaine, son propre lien disparaît à 1280 px.
 */
function WeekSummary({ week }: { week: WeekResponse }) {
  const texts = week.texts_count
  return (
    <section aria-labelledby="semaine" className="max-xl:band max-xl:py-8">
      <div className="flex items-baseline justify-between gap-3 xl:block">
        <h3 id="semaine" className="heading-4">
          {week.is_current ? 'Cette semaine' : 'Dernière semaine de votes'}
        </h3>
        <p className="text-[13px] text-ink-faint md:text-sm xl:mt-1">{weekSpan(week.monday)}</p>
      </div>
      <div className="mt-4 md:mt-5 md:grid md:grid-cols-[minmax(0,380px)_minmax(0,1fr)] md:items-end md:gap-x-10 xl:mt-4 xl:block">
        <WeekFigures totals={week.totals} large />
        <div className="mt-5 md:mt-0 xl:mt-6">
          <DayBars days={week.days} />
        </div>
      </div>
      <p className="mt-5 text-[13px] leading-[18px] text-ink-faint">
        Sur {texts} texte{texts > 1 ? 's' : ''}
        {week.last_published &&
          ` · dernier vote publié le ${new Date(week.last_published + 'T00:00:00').toLocaleDateString('fr-FR', { day: 'numeric', month: 'long' })}`}
      </p>
      <Link to="/semaine" className="mt-1 inline-flex min-h-11 items-center text-sm font-medium link xl:hidden">
        Tous les votes de la semaine
      </Link>
    </section>
  )
}

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
  // Même clé que la page La semaine : y aller depuis l'accueil ne recharge rien.
  const week = useQuery({
    queryKey: ['week', 'current'],
    queryFn: () => fetchJson<WeekResponse>('/api/semaine'),
    enabled: !legacyEntry,
    retry: false,
  })

  const usagesRef = useReveal<HTMLElement>()
  const reglesRef = useReveal<HTMLElement>()
  const piecesRef = useReveal<HTMLElement>()
  const sourcesRef = useReveal<HTMLElement>()

  if (legacyEntry) return <Navigate to={`${ENTRY_PATH}${location.search}`} replace />

  const vote = latest.data?.items[0]
  const groups = latest.data?.groups ?? []
  const card = vote ? finalVoteCard(vote, groups) : null

  return (
    <div className="flex flex-col gap-11 md:gap-14">
      {/* À partir de 1280 px : l'accroche en bande (boutons à droite), le dernier
          vote en pleine largeur, puis la semaine et ce qui devient loi côte à côte
          (variante C du 22/11/2026). Sous 1280 px, la semaine est une bande grise ;
          à partir de 1280 px, la bande les réunit. */}
      <section
        aria-labelledby="promesse"
        className="rise-in md:pt-6 xl:grid xl:grid-cols-[minmax(0,1fr)_auto] xl:gap-x-10 xl:pt-2"
      >
        <p className="text-[13px] font-medium text-ink-soft">Assemblée nationale</p>
        <h2
          id="promesse"
          className="mt-3 heading-1 max-sm:text-[38px] max-sm:leading-[40px] sm:text-[58px] sm:leading-[58px] md:max-w-[780px] md:text-[64px] md:leading-[66px] xl:mt-2 xl:text-[50px] xl:leading-[54px]"
        >
          Les votes de l’Assemblée, sur pièces.
        </h2>
        <p className="mt-4 max-w-xl text-base leading-relaxed text-ink-soft xl:max-w-[640px] xl:text-lg">
          Ce que chaque groupe a voté, texte par texte. Des chiffres bruts et la source
          officielle, sans sélection ni commentaire.
        </p>
        <div className="mt-5 flex flex-col items-start gap-1 sm:flex-row sm:flex-wrap sm:items-stretch sm:gap-2 xl:col-start-2 xl:row-span-2 xl:row-start-1 xl:mt-0 xl:flex-nowrap xl:justify-end xl:self-end">
          <Link
            to="/semaine"
            className={`group inline-flex min-h-[64px] items-center justify-between gap-3 self-stretch rounded-2xl bg-accent px-5 py-2 text-surface transition-colors hover:bg-accent-strong sm:justify-center sm:self-auto ${FOCUS_RING}`}
          >
            <span className="flex flex-col text-left">
              <span className="text-[15px] leading-[21px] font-semibold">Votes de la semaine</span>
              {week.data && (
                <span className="text-[13px] leading-[18px] font-medium text-surface/80">
                  {weekSpan(week.data.monday)} · {votesWord(week.data.totals.votes)}
                </span>
              )}
            </span>
            <Arrow />
          </Link>
          {/* Sous 768 px, un lien de texte ; au-delà, le bouton gris. */}
          <Link
            to="/candidats-2027"
            className={`group inline-flex min-h-11 items-center gap-2 text-[15px] font-semibold link sm:basis-full md:min-h-[64px] md:basis-auto md:justify-center md:rounded-2xl md:bg-surface md:px-5 md:no-underline md:transition-colors md:hover:bg-surface-soft ${FOCUS_RING}`}
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
        {vote && card ? (
          <VoteCard
            size="full"
            wide
            outcomeInFigure
            {...card}
            figure={
              <LatestVoteHemicycle
                scrutinUid={vote.scrutin_uid}
                tally={vote.synthesis}
                labelledBy="dernier-vote"
                centre={
                  <VoteIssue
                    adopted={card.adopted}
                    outcome={card.outcome}
                    outcomeTitle={card.outcomeTitle}
                    reading={card.reading}
                  />
                }
              />
            }
          />
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

      <div className="grid gap-11 empty:hidden md:gap-14 xl:band xl:grid-cols-[400px_minmax(0,1fr)] xl:items-start xl:gap-x-6 xl:py-10">
        {week.data && <WeekSummary week={week.data} />}
        <BecomingLawCard />
      </div>

      <section ref={usagesRef} aria-labelledby="usages">
        <h3 id="usages" className="heading-4 md:heading-2">
          À quoi ça sert
        </h3>
        <ul className="mt-2 md:mt-5 md:grid md:grid-cols-2 md:gap-x-6 lg:grid-cols-4">
          {USAGES.map((usage, index) => (
            <li key={usage.titre} className="reveal-item" style={revealRank(index)}>
              <Link
                to={usage.to}
                className={`group flex items-center gap-3 border-t border-line py-3.5 md:h-full md:items-start md:pt-[18px] md:pb-3 ${FOCUS_RING}`}
              >
                <span className="min-w-0 flex-1">
                  <span className="block text-[15px] leading-snug font-semibold text-ink group-hover:underline md:text-base">
                    {usage.titre}
                  </span>
                  <span className="mt-0.5 block text-[13px] leading-snug text-ink-soft md:mt-1.5 md:text-sm md:leading-normal">
                    {usage.texte}
                  </span>
                </span>
                <Arrow className="size-[18px] shrink-0 text-accent md:mt-0.5" />
              </Link>
            </li>
          ))}
        </ul>
      </section>

      {/* Sous 768 px : une bande pour les règles, puis les refus à plat. Au-delà :
          une seule bande, règles et refus côte à côte. À partir de 1280 px, une
          troisième colonne reprend les sources et les pièces, dont les sections
          suivantes disparaissent. */}
      <div className="md:band md:py-10 xl:py-11">
        <h3 className="hidden heading-2 xl:mb-7 xl:block">Sur pièces, et rien d’autre</h3>
        <section
          ref={reglesRef}
          aria-labelledby="regles"
          className="flex flex-col gap-11 md:grid md:grid-cols-2 md:gap-0 xl:grid-cols-3"
        >
          <div className="reveal-item max-md:band max-md:py-8 md:pr-8 xl:pr-10" style={revealRank(0)}>
            <h3 id="regles" className="heading-4">
              Trois règles
            </h3>
            <ol className="mt-4 space-y-3.5 [counter-reset:rule]">
              {REGLES.map((regle) => (
                <li
                  key={regle.titre}
                  className="grid grid-cols-[28px_minmax(0,1fr)] gap-2.5 [counter-increment:rule] before:font-heading before:text-[22px] before:leading-none before:font-[560] before:text-accent before:content-[counter(rule)]"
                >
                  <span className="text-sm leading-normal text-ink-soft">
                    <b className="block font-semibold text-ink">{regle.titre}</b> {regle.texte}
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
            </ol>
          </div>
          <div className="reveal-item md:border-l md:border-line md:pl-8 xl:px-8" style={revealRank(1)}>
            <h4 className="heading-4">Ce que ce site ne fait pas</h4>
            <ul className="mt-4 space-y-2.5">
              {REFUS.map((refus) => (
                <li key={refus} className="flex items-start gap-2.5 text-sm leading-[1.45] text-ink-soft">
                  <XMark />
                  {refus}
                </li>
              ))}
            </ul>
          </div>
          <div
            className="reveal-item hidden xl:block xl:border-l xl:border-line xl:pl-8"
            style={revealRank(2)}
          >
            <h4 className="heading-4">Tout se vérifie</h4>
            <ul className="mt-4">
              {VERIFIABLE.map((item) => (
                <li key={item.titre} className="border-t border-line py-3.5 first:border-t-0 first:pt-0">
                  <p className="font-semibold text-ink">{item.titre}</p>
                  <p className="mt-0.5 text-sm leading-normal text-ink-soft">{item.texte}</p>
                  {item.external ? (
                    <a href={item.to} target="_blank" rel="noreferrer" className={`mt-1 inline-block ${EXTERNAL_LINK}`}>
                      {item.label}
                    </a>
                  ) : (
                    <Link to={item.to} className={`mt-1 inline-block ${EXTERNAL_LINK}`}>
                      {item.label}
                    </Link>
                  )}
                </li>
              ))}
            </ul>
          </div>
        </section>
      </div>
      <div className="hidden xl:block">
        <Credits className="border-t border-line pt-3.5" />
      </div>

      <section ref={piecesRef} aria-labelledby="pieces" className="xl:hidden">
        <SectionHead
          id="pieces"
          surtitre="Derrière chaque vote"
          titre="Un décompte ne dit pas les raisons. Les pièces, si."
          chapo="Le site ne reconstitue pas les motivations d’un groupe et ne les résume pas à sa place : il rassemble ce qui est attribué, daté et public autour de chaque vote."
          info={NEVER_SHOWN}
        />
        <ol className="mt-5 grid sm:grid-cols-2 sm:gap-x-6 lg:grid-cols-4">
          {PIECES.map((piece, index) => (
            <li key={piece.titre} className="reveal-item" style={revealRank(index)}>
              <div className="h-full border-t border-line py-3.5">
                <h4 className="flex items-center gap-2.5 text-[15px] font-semibold">
                  <span className="font-heading text-[22px] leading-none font-[560] text-accent tabular-nums">
                    {index + 1}
                  </span>
                  {piece.titre}
                </h4>
                <p className="mt-1 text-[13px] leading-snug text-ink-soft">{piece.texte}</p>
              </div>
            </li>
          ))}
        </ol>
        <div className="mt-3">
          <Note desktopOnly>{NEVER_SHOWN}</Note>
        </div>
      </section>

      <section ref={sourcesRef} aria-labelledby="sources" className="xl:hidden">
        <SectionHead id="sources" surtitre="Sources" titre="Tout vient de l’Assemblée, et se vérifie" />
        <div className="mt-5 grid md:grid-cols-3 md:gap-x-8">
          <div className="border-t border-line py-4">
            <h4 className="text-[15px] font-semibold">Données</h4>
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
          <div className="border-t border-line py-4">
            <h4 className="text-[15px] font-semibold">Code source</h4>
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
          <div className="border-t border-line py-4">
            <h4 className="text-[15px] font-semibold">Vocabulaire</h4>
            <p className="mt-1 text-[13.5px] leading-relaxed text-ink-soft">
              Non-votant, abstention, scrutin solennel, motion de censure : chaque mot de la
              procédure est expliqué, en deux niveaux de lecture.
            </p>
            <Link to="/comprendre" className={`mt-1.5 inline-block ${EXTERNAL_LINK}`}>
              Comprendre
            </Link>
          </div>
        </div>
        <Credits className="border-t border-line pt-3.5" />
      </section>
    </div>
  )
}
