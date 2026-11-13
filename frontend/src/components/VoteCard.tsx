import { useCallback, useState, type CSSProperties, type ReactNode } from 'react'
import { Link } from 'react-router'
import { GROUP_LOGOS } from '../content/logos'
import {
  capitalized,
  findStance,
  groupPosition,
  groupShortName,
  stanceDescription,
  stanceSentence,
  type GroupPosition,
} from '../lib/entryFlow'
import type { GroupDto, StanceDto, TallyDto } from '../types/finalVotes'
import { formatDate } from '../types/scrutins'
import GroupBreakdown from './entry/GroupBreakdown'
import PositionGlyph from './entry/PositionGlyph'
import { BADGE_CLASS, BADGE_WORD, BANNER_CLASS, FOCUS_RING, SWATCH_CLASS } from './entry/styles'
import { PoliticalLogo } from './PoliticalLogo'
import { VoteBar } from './ui'

/*
 * Carte de vote unique du site, en deux tailles.
 *
 * Même en-tête partout (l'issue et la lecture dans une pastille, puis la date),
 * même pied partout (la barre et le décompte de toute l'Assemblée). Seul le
 * milieu varie : les groupes rangés par position (`full`), ou rien
 * (`compact`), pour les listes longues.
 *
 * Le titre est l'intitulé publié par l'Assemblée, jamais reformulé
 * (README.md §6).
 */

/** Positions des groupes sur ce vote, et les groupes du référentiel à placer. */
export interface VoteCardGroups {
  /** Groupes dans l'ordre de l'hémicycle. */
  groups: GroupDto[]
  stances: StanceDto[]
  /** Sigles suivis ; vide = tous. */
  followed?: string[]
}

export interface VoteCardProps {
  size: 'full' | 'compact'
  /** Fiche du vote. */
  to: string
  adopted: boolean
  /** Issue courte, accordée au texte voté : « Adoptée », « Rejeté ». */
  outcome: string
  /** Libellé publié de l'issue, en infobulle. */
  outcomeTitle?: string
  reading?: string | null
  date: string
  /** Ce qui a été mis aux voix, au-dessus du titre : « Amendement n° 655 · Article 45 ». */
  kicker?: ReactNode
  /** Intitulé officiel. */
  title: string
  author?: ReactNode
  excerpt?: string | null
  /** Mentions sous le titre (procédure, répartition reconstituée…). */
  notes?: ReactNode
  tally: TallyDto
  /** Carte complète seulement. */
  groups?: VoteCardGroups
  /** Sans cadre : la carte est une ligne d'un bloc qui l'encadre déjà. */
  bare?: boolean
  /** Carte seule en pleine largeur : à partir de 1024 px, l'intitulé à gauche,
   * les groupes et le décompte à droite. */
  wide?: boolean
  /** Carte large seulement : cette figure (l'hémicycle de l'accueil) passe sous
   * l'intitulé, au-dessus des groupes ; à partir de 1024 px, l'intitulé prend
   * toute la largeur et la figure se place à gauche des groupes. */
  figure?: ReactNode
}

const plural = (count: number, word: string) => `${count} ${word}${count > 1 ? 's' : ''}`

function tallySentence(tally: TallyDto): string {
  return [
    `Toute l’Assemblée : ${tally.votes_for} pour`,
    `${tally.votes_against} contre`,
    plural(tally.abstentions, 'abstention'),
    ...(tally.not_voting > 0 ? [plural(tally.not_voting, 'non-votant')] : []),
  ].join(', ')
}

/** Pied commun : la barre, puis les chiffres bruts de toute l'Assemblée. */
export function AssemblyFooter({ tally }: { tally: TallyDto }) {
  return (
    <div className="border-t border-line pt-2.5">
      <VoteBar tally={{ ...tally, not_voting: 0 }} label={tallySentence(tally)} />
      <p className="mt-1.5 text-xs text-ink-soft tabular-nums">
        Toute l’Assemblée : <b className="font-semibold text-yes">{tally.votes_for}</b> pour ·{' '}
        <b className="font-semibold text-no">{tally.votes_against}</b> contre ·{' '}
        <b className="font-semibold text-abstain">{tally.abstentions}</b> abst.
        {tally.not_voting > 0 && <> · {plural(tally.not_voting, 'non-votant')}</>}
      </p>
    </div>
  )
}

/** En-tête commun : l'issue et la lecture dans une pastille, puis la date. */
export function VoteCardHeader({
  adopted,
  outcome,
  outcomeTitle,
  reading,
  date,
}: Pick<VoteCardProps, 'adopted' | 'outcome' | 'outcomeTitle' | 'reading' | 'date'>) {
  return (
    <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[12.5px] text-ink-faint">
      <span
        title={outcomeTitle}
        className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 font-semibold ${
          adopted ? 'bg-yes-soft text-yes' : 'bg-no-soft text-no'
        }`}
      >
        <PositionGlyph position={adopted ? 'for' : 'against'} className="size-3" strokeWidth={3.2} />
        {outcome}
        {reading && <> · {readingShort(reading)}</>}
      </span>
      <time dateTime={date}>{formatDate(date)}</time>
    </p>
  )
}

/** « première lecture » → « 1re lecture » : la pastille doit tenir sur une ligne. */
function readingShort(reading: string): string {
  const ordinals: Record<string, string> = {
    première: '1re',
    deuxième: '2e',
    troisième: '3e',
    nouvelle: 'Nouvelle',
  }
  const [first, ...rest] = reading.trim().split(/\s+/)
  const short = ordinals[first.toLowerCase()]
  return short ? [short, ...rest].join(' ') : capitalized(reading)
}

/* ------------------------------------------------------------- groupes */

/**
 * Rangées de la carte complète. Un groupe partagé à égalité et un groupe sans
 * ligne publiée ont leur rangée : rien n'est rangé d'office dans une position
 * qu'il n'a pas prise (README.md §2).
 */
const ROWS: { key: GroupPosition; label: string; matches: GroupPosition[] }[] = [
  { key: 'for', label: 'Pour', matches: ['for'] },
  { key: 'against', label: 'Contre', matches: ['against'] },
  { key: 'abstention', label: 'Abstention', matches: ['abstention'] },
  { key: 'split', label: 'Partagés', matches: ['split'] },
  { key: 'nobody', label: 'Aucun votant', matches: ['nobody'] },
  { key: 'missing', label: 'Non publié', matches: ['missing'] },
]

const ROW_WORD_CLASS: Record<GroupPosition, string> = {
  for: 'text-yes',
  against: 'text-no',
  abstention: 'text-abstain',
  split: 'text-info',
  nobody: 'text-ink-soft',
  missing: 'text-ink-soft',
}

/** Bandeau du groupe suivi quand un seul l'est : sa position, puis ses chiffres bruts. */
function FollowedBanner({ group, stance }: { group: GroupDto; stance: StanceDto | undefined }) {
  const position = groupPosition(stance)
  return (
    <div
      className={`flex flex-wrap items-center gap-x-3 gap-y-2 rounded px-3 py-2.5 ${BANNER_CLASS[position]}`}
    >
      <span
        className={`min-w-[84px] shrink-0 rounded px-3 py-1.5 text-center text-[15px] font-bold tracking-wide uppercase ${BADGE_CLASS[position]}`}
      >
        {BADGE_WORD[position]}
      </span>
      <p className="min-w-0 flex-1 text-[15px] font-semibold">
        <span className="sr-only">{groupShortName(group)} : </span>
        {stanceSentence(stance)}
      </p>
    </div>
  )
}

/**
 * Les groupes rangés par position, dans l'ordre de l'hémicycle à l'intérieur de
 * chaque rangée. Toucher un logo ouvre la répartition du groupe. Les groupes
 * suivis portent un anneau ; les autres s'estompent sans disparaître.
 */
function GroupsByPosition({
  groups,
  stances,
  followed = [],
  wide = false,
}: VoteCardGroups & { wide?: boolean }) {
  const [opened, setOpened] = useState<{ group: GroupDto; anchor: HTMLElement } | null>(null)
  const close = useCallback(() => setOpened(null), [])
  const dimOthers = followed.length > 0
  const vote = { stances }
  const placed = groups.map((group) => {
    const stance = findStance(vote, group)
    return { group, stance, position: groupPosition(stance) }
  })
  const followedGroups = groups.filter((group) => followed.includes(group.abbrev))
  const focus = followedGroups.length === 1 ? followedGroups[0] : null
  // Rang de chaque logo dans l'ordre de lecture, toutes rangées confondues :
  // il règle son retard d'arrivée (`.tiles-in`, index.css).
  let tile = 0

  return (
    <div className="space-y-2.5">
      {focus && <FollowedBanner group={focus} stance={findStance(vote, focus)} />}
      <div>
        {ROWS.map((row) => {
          const members = placed.filter((entry) => row.matches.includes(entry.position))
          if (members.length === 0) return null
          return (
            <div
              key={row.key}
              // En large, la colonne tient « Aucun votant » sur une ligne.
              className={`grid grid-cols-[92px_minmax(0,1fr)] items-start gap-2 border-t border-line py-2 ${
                wide ? 'lg:grid-cols-[112px_minmax(0,1fr)]' : ''
              }`}
            >
              <div className="flex min-h-[30px] items-center gap-1.5">
                <span
                  aria-hidden
                  className={`inline-flex size-5 shrink-0 items-center justify-center rounded ${SWATCH_CLASS[row.key]}`}
                >
                  <PositionGlyph position={row.key} className="size-3" />
                </span>
                <span className={`text-[13px] font-semibold ${ROW_WORD_CLASS[row.key]}`}>
                  {row.label}
                </span>
              </div>
              <ul className="tiles-in flex flex-wrap gap-1.5" aria-label={row.label}>
                {members.map(({ group, stance }) => {
                  const on = followed.includes(group.abbrev)
                  const isOpen = opened?.group.uid === group.uid
                  const description = stanceDescription(group, stance)
                  return (
                    <li key={group.uid} style={{ '--tile': tile++ } as CSSProperties}>
                      <button
                        type="button"
                        aria-label={description}
                        aria-haspopup="dialog"
                        aria-expanded={isOpen}
                        title={description}
                        onClick={(event) =>
                          setOpened(isOpen ? null : { group, anchor: event.currentTarget })
                        }
                        // Le logo reste à 30 px de haut, la zone de toucher atteint 44 px.
                        className={`relative flex h-[30px] w-11 items-center justify-center rounded bg-white p-[3px] ring-1 ring-black/10 ring-inset transition-[opacity,transform] motion-safe:hover:-translate-y-0.5 after:absolute after:-inset-y-[7px] after:inset-x-0 after:content-[''] ${FOCUS_RING} ${
                          on ? 'ring-2 ring-ink' : ''
                        } ${isOpen ? 'outline-2 outline-offset-1 outline-accent' : ''} ${
                          dimOthers && !on && !isOpen ? 'opacity-35 hover:opacity-80' : ''
                        }`}
                      >
                        <PoliticalLogo
                          logo={GROUP_LOGOS[group.abbrev]}
                          fallback={group.abbrev}
                          color={null}
                          className="h-full w-full"
                        />
                      </button>
                    </li>
                  )
                })}
              </ul>
            </div>
          )
        })}
      </div>
      {opened && (
        <GroupBreakdown
          group={opened.group}
          stance={findStance(vote, opened.group)}
          anchor={opened.anchor}
          onClose={close}
        />
      )}
    </div>
  )
}

/* ---------------------------------------------------------------- carte */

export default function VoteCard({
  size,
  to,
  adopted,
  outcome,
  outcomeTitle,
  reading,
  date,
  kicker,
  title,
  author,
  excerpt,
  notes,
  tally,
  groups,
  bare = false,
  wide = false,
  figure,
}: VoteCardProps) {
  const compact = size === 'compact'
  const withFigure = wide && figure != null
  const results = (
    <div className="flex flex-col gap-2.5">
      {!compact && groups && <GroupsByPosition {...groups} wide={wide} />}
      <AssemblyFooter tally={tally} />
    </div>
  )
  // En compacte, rien d'autre ne se touche : le lien du titre couvre la carte.
  const stretched = compact ? 'after:absolute after:inset-0 after:content-[\'\']' : ''

  return (
    <article
      className={`group relative flex flex-col gap-2.5 ${
        bare
          ? 'px-4 py-3.5 sm:px-[18px]'
          : 'rounded border border-line bg-surface p-4 shadow-card sm:px-5'
      } ${compact ? 'transition-colors hover:bg-surface-soft/50' : ''} ${
        withFigure ? 'lg:px-7 lg:py-6' : ''
      } ${
        wide && !withFigure
          ? 'lg:grid lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] lg:items-start lg:gap-x-10 lg:px-7 lg:py-6'
          : ''
      }`}
    >
      <div className="space-y-1.5">
        <VoteCardHeader
          adopted={adopted}
          outcome={outcome}
          outcomeTitle={outcomeTitle}
          reading={reading}
          date={date}
        />
        {kicker && <p className="text-[12.5px] font-medium text-ink-soft">{kicker}</p>}
        <h3
          className={`text-ink ${
            compact
              ? 'text-[15px] leading-snug font-semibold tracking-tight'
              : `heading-5 ${wide ? 'lg:text-2xl lg:leading-[30px]' : ''}`
          }`}
        >
          <Link
            to={to}
            className={`rounded transition-colors hover:text-accent group-hover:text-accent ${FOCUS_RING} ${stretched}`}
          >
            {title}
          </Link>
        </h3>
        {author && <div>{author}</div>}
        {excerpt && (
          <p className="line-clamp-3 border-l-[3px] border-line-strong pl-3 text-[13px] leading-relaxed text-ink-soft">
            {excerpt}
          </p>
        )}
        {notes}
      </div>

      {withFigure ? (
        <div className="lg:mt-[22px] lg:grid lg:grid-cols-2 lg:items-center lg:gap-x-12 lg:border-t lg:border-line lg:pt-[22px]">
          <div className="mt-2 mb-[18px] lg:my-0">{figure}</div>
          {results}
        </div>
      ) : (
        results
      )}
    </article>
  )
}
