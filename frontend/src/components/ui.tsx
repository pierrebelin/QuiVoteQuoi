import type { CSSProperties, ReactNode } from 'react'
import type { TallyDto } from '../types/scrutins'
import { Icon } from './icons'

/**
 * Primitives visuelles du site.
 *
 * Elles existent pour que les quinze pages parlent la même langue : une pastille
 * a partout la même taille, une barre de vote partout la même échelle. Une carte
 * qui compose ses classes à la main finit par diverger, et c'est la divergence
 * qui donne l'impression d'un assemblage plutôt que d'un produit.
 */

/* ------------------------------------------------------------------ titres */

/**
 * En-tête de page : un titre, une phrase qui dit ce que la page montre, et
 * éventuellement un contrôle aligné à droite.
 *
 * Le chapô n'est pas décoratif. La plupart des pages affichent des chiffres
 * dont la portée exacte n'est pas devinable — « tous les scrutins publics, sans
 * sélection » est une garantie (README.md §2), pas un slogan.
 */
export function PageHeader({
  title,
  lede,
  aside,
}: {
  title: string
  lede?: ReactNode
  aside?: ReactNode
}) {
  return (
    <div className="mb-5 flex flex-wrap items-end justify-between gap-x-6 gap-y-2">
      <div className="min-w-0">
        <h2 className="text-2xl font-semibold tracking-tight text-ink">
          {title}
        </h2>
        {lede && (
          <p className="mt-1 text-sm leading-relaxed text-ink-soft">
            {lede}
          </p>
        )}
      </div>
      {aside && <div className="shrink-0">{aside}</div>}
    </div>
  )
}

/** Intitulé de section, au-dessus d'une liste ou d'une grille. */
export function SectionTitle({
  children,
  count,
}: {
  children: ReactNode
  count?: ReactNode
}) {
  return (
    <div className="mb-2 flex items-baseline gap-2">
      <h3 className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
        {children}
      </h3>
      {count != null && <span className="text-xs text-ink-faint">{count}</span>}
    </div>
  )
}

/* ------------------------------------------------------------- conteneurs */

/** Carte : fond, trait fin, ombre d'un pixel. Aucun dégradé, aucun relief. */
export function Card({
  children,
  className = '',
  style,
}: {
  children: ReactNode
  className?: string
  style?: CSSProperties
}) {
  return (
    <div
      className={`rounded-xl border border-line bg-surface shadow-card ${className}`}
      style={style}
    >
      {children}
    </div>
  )
}

/**
 * Liste de lignes cliquables. Le trait de séparation vient du conteneur, pas
 * de chaque ligne : une ligne ne connaît pas sa position dans la liste.
 */
export function ListCard({
  children,
  muted = false,
}: {
  children: ReactNode
  muted?: boolean
}) {
  return (
    <div
      className={`divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card transition-opacity ${
        muted ? 'opacity-50' : ''
      }`}
    >
      {children}
    </div>
  )
}

/** Encadré de méthode ou de lacune. Discret, jamais masqué (README.md §2). */
export function Note({ children }: { children: ReactNode }) {
  return (
    <div className="rounded-lg border border-line bg-surface-soft px-3 py-2 text-xs leading-relaxed text-ink-soft">
      {children}
    </div>
  )
}

export function ErrorPanel({ error }: { error: unknown }) {
  return (
    <div className="rounded-xl border border-no/25 bg-no-soft px-4 py-3">
      <p className="text-sm text-no">
        Erreur : {error instanceof Error ? error.message : 'inconnue'}
      </p>
    </div>
  )
}

export function Loading({ children }: { children: ReactNode }) {
  return <p className="animate-pulse py-10 text-sm text-ink-faint">{children}</p>
}

/* ---------------------------------------------------------------- pastilles */

export type PillTone = 'yes' | 'no' | 'neutral' | 'info' | 'abstain'

const pillTones: Record<PillTone, string> = {
  yes: 'bg-yes-soft text-yes ring-yes/15',
  no: 'bg-no-soft text-no ring-no/15',
  abstain: 'bg-abstain-soft text-abstain ring-abstain/15',
  info: 'bg-info-soft text-info ring-info/15',
  neutral: 'bg-surface-soft text-ink-soft ring-line',
}

/**
 * Pastille d'état. Un anneau d'un pixel plutôt qu'une bordure : sur un fond
 * teinté, la bordure pleine épaissit la pastille et la fait ressembler à un
 * bouton cliquable.
 */
export function Pill({
  children,
  tone = 'neutral',
  title,
}: {
  children: ReactNode
  tone?: PillTone
  title?: string
}) {
  return (
    <span
      title={title}
      className={`inline-flex shrink-0 items-center rounded-md px-1.5 py-0.5 text-[11px] font-medium leading-4 ring-1 ring-inset ${pillTones[tone]}`}
    >
      {children}
    </span>
  )
}

/** Métadonnée en ligne : n° de scrutin, date, procédure. */
export function Meta({
  children,
  className = '',
}: {
  children: ReactNode
  className?: string
}) {
  return (
    <span className={`text-xs text-ink-faint ${className}`}>{children}</span>
  )
}

/* ------------------------------------------------------------ barre de vote */

type Segment = { key: string; value: number; className: string; label: string }

function segments(tally: TallyDto): Segment[] {
  const notVoting = tally.not_voting + tally.voluntary_not_voting
  return [
    { key: 'for', value: tally.votes_for, className: 'bg-yes-bar', label: 'pour' },
    { key: 'against', value: tally.votes_against, className: 'bg-no-bar', label: 'contre' },
    { key: 'abstention', value: tally.abstentions, className: 'bg-abstain-bar', label: 'abstention' },
    { key: 'not_voting', value: notVoting, className: 'bg-line-strong', label: 'non-votant' },
  ]
}

/**
 * Répartition d'un scrutin, en une barre.
 *
 * Un seul rail à quatre segments, pas quatre barres empilées : les segments se
 * lisent les uns par rapport aux autres sans arithmétique. Les non-votants
 * occupent leur part réelle — les écarter gonflerait mécaniquement le reste.
 *
 * `outOf` sert à comparer plusieurs barres entre elles. Sans lui, chaque barre
 * se normalise sur son propre total : côte à côte, un groupe de quarante-neuf
 * votants et un groupe de soixante-neuf occupent la même longueur, ce qui donne
 * à voir une égalité de poids qui n'existe pas. En passant le plus grand total
 * de la série, la longueur dit l'effectif et les segments disent la répartition.
 *
 * La barre ne remplace jamais les chiffres, elle les accompagne : une longueur
 * ne se lit pas, elle s'estime (README.md §6, §8).
 */
export function VoteBar({
  tally,
  outOf,
  label,
}: {
  tally: TallyDto
  outOf?: number
  /** Remplace la phrase par défaut, lue et affichée en infobulle. */
  label?: string
}) {
  const parts = segments(tally)
  const total = parts.reduce((sum, part) => sum + part.value, 0)
  if (total === 0) return null

  const scale = outOf && outOf > 0 ? Math.min(1, total / outOf) : 1

  return (
    <div className="w-full rounded-full bg-surface-soft">
      <div
        className="flex h-1.5 overflow-hidden rounded-full"
        style={{ width: `${scale * 100}%` }}
        role="img"
        title={label}
        aria-label={
          label ??
          parts
            .filter((part) => part.value > 0)
            .map(
              (part) => `${part.value} ${part.label}${part.value > 1 ? 's' : ''}`,
            )
            .join(', ')
        }
      >
        {parts
          .filter((part) => part.value > 0)
          .map((part) => (
            <span
              key={part.key}
              className={part.className}
              style={{ width: `${(part.value / total) * 100}%` }}
            />
          ))}
      </div>
    </div>
  )
}

/**
 * Décompte brut, jamais d'adverbe : « 280 contre 250 », pas « massivement ».
 *
 * `dense` réduit la ligne aux trois positions exprimées, pour les listes où la
 * ligne complète déborderait. Les non-votants restent lisibles sur la page du
 * scrutin, qui est l'endroit où on vient les chercher.
 */
export function TallyLine({
  tally,
  dense = false,
}: {
  tally: TallyDto
  dense?: boolean
}) {
  const notVoting = tally.not_voting + tally.voluntary_not_voting
  return (
    <span className="text-xs text-ink-faint">
      <span className="font-semibold text-yes">{tally.votes_for}</span> pour
      <span className="px-1 text-line-strong">·</span>
      <span className="font-semibold text-no">{tally.votes_against}</span> contre
      <span className="px-1 text-line-strong">·</span>
      <span className="font-semibold text-abstain">{tally.abstentions}</span> abst.
      {!dense && notVoting > 0 && (
        <>
          <span className="px-1 text-line-strong">·</span>
          {notVoting} non-votant{notVoting > 1 ? 's' : ''}
        </>
      )}
    </span>
  )
}

/* -------------------------------------------------------------- contrôles */

/**
 * Groupe de filtres. Le segment actif est plein plutôt que teinté : sur une
 * ligne de quatre groupes, une teinte pâle ne se distingue plus du survol. Il
 * porte aussi une coche et un trait, pour que l'état ne repose pas sur la seule
 * couleur, et reste net en sombre où le fond de la surface est proche de celui
 * de la piste.
 */
export function SegmentedControl({
  label,
  options,
  value,
  onChange,
}: {
  label?: string
  options: { value: string; label: string }[]
  value: string
  onChange: (value: string) => void
}) {
  return (
    <div className="flex min-w-0 items-center gap-2">
      {label && (
        <span className="shrink-0 text-xs font-medium text-ink-faint">
          {label}
        </span>
      )}
      <div className="flex min-w-0 flex-wrap gap-0.5 rounded-lg bg-surface-soft p-0.5">
        {options.map((option) => {
          const active = value === option.value
          return (
            <button
              key={option.value}
              type="button"
              aria-pressed={active}
              onClick={() => onChange(option.value)}
              className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                active
                  ? 'bg-surface text-ink shadow-card ring-1 ring-line-strong ring-inset'
                  : 'text-ink-soft hover:text-ink'
              }`}
            >
              {active && <Icon name="check" className="size-3" strokeWidth={3} />}
              {option.label}
            </button>
          )
        })}
      </div>
    </div>
  )
}

export function Button({
  children,
  onClick,
  type = 'button',
  disabled = false,
}: {
  children: ReactNode
  onClick?: () => void
  type?: 'button' | 'submit'
  disabled?: boolean
}) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className="rounded-lg border border-line bg-surface px-3 py-1.5 text-sm font-medium text-ink-soft shadow-card transition-colors hover:bg-surface-soft hover:text-ink disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-surface"
    >
      {children}
    </button>
  )
}

/**
 * Grand chiffre commenté. Sert à poser une volumétrie sans la noyer dans une
 * phrase — le nombre est la donnée, le libellé n'est que sa légende.
 */
export function Stat({
  value,
  label,
  hint,
}: {
  value: ReactNode
  label: string
  hint?: string
}) {
  return (
    <div>
      <p className="text-xl font-semibold tracking-tight text-ink">{value}</p>
      <p className="text-xs text-ink-soft">{label}</p>
      {hint && <p className="mt-0.5 text-[11px] text-ink-faint">{hint}</p>}
    </div>
  )
}
