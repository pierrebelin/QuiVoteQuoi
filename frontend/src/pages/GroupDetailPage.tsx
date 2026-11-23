import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { InfoTip } from '../components/InfoTip'
import { BackLink, ErrorPanel, Loading, Note, Pill } from '../components/ui'
import { formatDate } from '../types/scrutins'
import {
  formatCount,
  formatPerMille,
  type GroupDetailResponse,
  type ParticipationRatesDto,
} from '../types/groups'
import { GroupDot } from './GroupListPage'

/**
 * Couleurs des trois parts.
 *
 * Ni vert ni rouge, contrairement aux pages de vote : là-bas ils disent « pour »
 * et « contre », ici ils diraient « bien » et « mal ». Une participation élevée
 * n'est pas un mérite et une abstention n'est pas une faute (README.md §6).
 * Refonte A : encre pour les voix exprimées, ambre de l'abstention, gris pour
 * la non-participation, plus de bleu.
 */
const RATE_BARS = {
  expressed: 'bg-ink-soft',
  abstention: 'bg-abstain-bar',
  absence: 'bg-line-strong',
} as const

/** Les mises en garde dues (README.md §6), lues au même endroit quel que soit l'écran. */
function GroupNotes({
  group,
  className = 'space-y-1',
}: {
  group: GroupDetailResponse
  className?: string
}) {
  return (
    <div className={className}>
      <p>{group.party_note}</p>
      <p>{group.rate_note}</p>
      <p>{group.comparison_note}</p>
      <p>{group.hand_vote_note}</p>
      {group.reconstructed_count > 0 && <p>{group.reconstructed_note}</p>}
      {group.silent_line_count > 0 && <p>{group.silent_line_note}</p>}
    </div>
  )
}

/**
 * Section à plat (refonte A) : un titre en Bricolage, pas de carte. `label`
 * ramène le titre à un intitulé discret sur téléphone (Identité, qui prolonge
 * l'en-tête) ; à partir de 768 px, tous les titres sont de même rang.
 */
function Section({
  title,
  info,
  label = false,
  className = '',
  children,
}: {
  title: string
  info?: React.ReactNode
  label?: boolean
  className?: string
  children: React.ReactNode
}) {
  return (
    <section className={className}>
      <h3
        className={
          label
            ? 'text-[13px] font-semibold text-ink-faint md:heading-3 md:text-ink'
            : 'heading-4 text-ink md:heading-3'
        }
      >
        {title}
        {info && (
          <InfoTip label={`Comment lire : ${title}`} className="md:hidden">
            {info}
          </InfoTip>
        )}
      </h3>
      <div className="mt-3 md:mt-4">{children}</div>
    </section>
  )
}

const FIGURE_SIZE = {
  26: 'text-[26px]',
  28: 'text-[28px]',
  34: 'text-[34px]',
  40: 'text-[40px]',
} as const

/**
 * Un relevé : l'intitulé d'abord dans le source, puis la valeur.
 *
 * - `row` : une ligne, l'intitulé à gauche, la valeur à droite ;
 * - `figure` : la valeur en grand au-dessus de l'intitulé (`size` en pixels) ;
 * - `text` : l'intitulé discret au-dessus de la valeur (en grand avec `size`).
 * Dans `figure`, la valeur passe au-dessus de l'intitulé par l'ordre
 * d'affichage seul : la lecture au clavier ou à l'écran garde l'intitulé d'abord.
 */
function Fact({
  label,
  variant = 'row',
  size,
  className = '',
  children,
}: {
  label: string
  variant?: 'row' | 'figure' | 'text'
  size?: keyof typeof FIGURE_SIZE
  className?: string
  children: React.ReactNode
}) {
  const big = size ? `figure ${FIGURE_SIZE[size]} text-ink` : ''
  if (variant === 'figure') {
    return (
      <div className={`flex min-w-0 flex-col-reverse justify-end gap-1.5 ${className}`}>
        <span className="text-[13.5px] leading-snug text-ink-soft">{label}</span>
        <span className={big || `figure ${FIGURE_SIZE[34]} text-ink`}>{children}</span>
      </div>
    )
  }
  if (variant === 'text') {
    return (
      <div className={`flex min-w-0 flex-col gap-1 ${className}`}>
        <span className="text-[13.5px] text-ink-soft">{label}</span>
        <span className={big || 'text-base leading-7 font-[620] text-ink'}>{children}</span>
      </div>
    )
  }
  return (
    <div
      className={`flex flex-wrap items-baseline justify-between gap-x-4 gap-y-0.5 border-t border-line py-3 ${className}`}
    >
      <span className="text-sm text-ink-soft">{label}</span>
      <span className="text-base font-[620] text-ink">{children}</span>
    </div>
  )
}

/** Barre des trois parts. Elle rend visible ce que le pourcentage résume. */
function RateBar({ rates }: { rates: ParticipationRatesDto }) {
  return (
    <div className="bar-grow flex h-3 w-full gap-0.5 overflow-hidden rounded-full bg-surface-soft">
      <div
        className={RATE_BARS.expressed}
        style={{ width: `${rates.expressed_per_mille / 10}%` }}
      />
      <div
        className={RATE_BARS.abstention}
        style={{ width: `${rates.abstention_per_mille / 10}%` }}
      />
      <div
        className={RATE_BARS.absence}
        style={{ width: `${rates.absence_per_mille / 10}%` }}
      />
    </div>
  )
}

/**
 * Un taux et le compte brut qui l'a produit. Le pourcentage ne remplace jamais
 * le chiffre : les deux sont affichés côte à côte (README.md §6).
 */
function Rate({
  label,
  swatch,
  perMille,
  count,
  detail,
}: {
  label: string
  swatch: string
  perMille: number
  count: number
  detail?: string
}) {
  return (
    <div className="grid grid-cols-[12px_minmax(0,1fr)_auto] items-start gap-x-3 border-t border-line py-3.5 md:grid-cols-[12px_minmax(0,1fr)] md:gap-x-2.5 md:gap-y-1 md:py-4">
      <span
        className={`col-start-1 row-span-2 mt-[5px] size-3 shrink-0 rounded-[3px] ring-1 ring-inset ring-chip-edge md:row-span-1 ${swatch}`}
        aria-hidden
      />
      <span className="col-start-2 text-base font-[620] text-ink">{label}</span>
      <span className="figure col-start-3 row-span-2 row-start-1 text-[30px] text-ink md:col-start-2 md:row-span-1 md:row-start-2 md:text-[44px]">
        {formatPerMille(perMille)}
      </span>
      <p className="col-start-2 row-start-2 text-[13px] text-ink-faint md:row-start-3 md:text-sm">
        {formatCount(count)} position{count > 1 ? 's' : ''}
        {detail && <> — {detail}</>}
      </p>
    </div>
  )
}

/**
 * Fiche d'un groupe parlementaire.
 *
 * Les taux décomposent la participation — voix exprimées, abstention,
 * non-participation — et jamais le sens du vote. « Pour » et « contre » sont
 * additionnés ensemble : cumulés sur toute la législature, dont 86 % de
 * scrutins d'amendement, ils ne décriraient aucune position et se liraient
 * pourtant comme une (README.md §6).
 */
export default function GroupDetailPage() {
  const { uid } = useParams<{ uid: string }>()

  const { data, isLoading, isError, error } = useQuery<GroupDetailResponse>({
    queryKey: ['groupes', uid],
    queryFn: () =>
      fetch(`/api/groupes/${encodeURIComponent(uid!)}`).then(async (res) => {
        if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`)
        return res.json()
      }),
  })

  if (isLoading) return <Loading>Chargement du groupe…</Loading>

  if (isError) {
    return (
      <div className="space-y-3">
        <ErrorPanel error={error} />
        <BackLink to="/groupes">Tous les groupes</BackLink>
      </div>
    )
  }

  const group = data!
  const { counts, rates } = group

  return (
    <div>
      <div>
        <BackLink to="/groupes">Tous les groupes</BackLink>
        <div className="mt-4 flex flex-wrap items-center gap-x-3 gap-y-1 md:mt-6">
          <GroupDot color={group.color} size="lg" />
          <h2 className="heading-2 md:heading-1">
            {group.abbrev}
          </h2>
          <span className="flex basis-full items-center gap-2 text-[19px] leading-6 font-semibold tracking-[-0.01em] text-ink md:basis-auto md:text-2xl">
            {group.label}
            {group.dissolved && <Pill>dissous</Pill>}
          </span>
        </div>
      </div>

      {/* Les trois relevés factuels tiennent de front : la largeur du site les
          porte, et empilés ils poussaient la participation hors de l'écran. */}
      <div className="mt-8 space-y-9 md:mt-10 md:grid md:grid-cols-3 md:space-y-0 md:divide-x md:divide-line [&>section]:md:px-8 [&>section:first-child]:md:pl-0 [&>section:last-child]:md:pr-0">
        <Section title="Identité" label>
          <div className="grid grid-cols-2 gap-x-6 gap-y-4 border-y border-line py-4 md:grid-cols-1 md:gap-y-0 md:divide-y md:divide-line md:border-y-0 md:py-0 [&>div]:md:py-4 [&>div:first-child]:md:pt-0">
            <Fact label="Législature" variant="text" size={28}>
              {group.legislature}
              <sup className="text-[15px]">e</sup>
            </Fact>
            <Fact label="Constitué le" variant="text">
              {group.created_on ? formatDate(group.created_on) : 'non publié'}
            </Fact>
            {group.dissolved_on && (
              <Fact label="Dissous le" variant="text">
                {formatDate(group.dissolved_on)}
              </Fact>
            )}
            {group.former_abbrevs.length > 0 && (
              <Fact label="Sigles antérieurs" variant="text">
                {group.former_abbrevs.join(', ')}
              </Fact>
            )}
          </div>
        </Section>

        <Section title="Effectif">
          <div className="grid grid-cols-2 gap-x-5">
            <Fact label={`Députés au ${formatDate(group.reference_date)}`} variant="figure">
              {formatCount(group.member_count)}
            </Fact>
            <Fact label="Députés y ayant siégé depuis sa constitution" variant="figure">
              {formatCount(group.total_member_count)}
            </Fact>
          </div>
          {group.published_member_range && (
            <Fact label="Effectif publié sur les votes" className="mt-5">
              {group.published_member_range.stable
                ? formatCount(group.published_member_range.min)
                : `de ${formatCount(group.published_member_range.min)} à ${formatCount(
                    group.published_member_range.max,
                  )}`}
            </Fact>
          )}
          {group.qualities.length > 0 && (
            <div
              className={`grid grid-cols-3 gap-x-3 gap-y-4 border-t border-line pt-4 ${
                group.published_member_range ? '' : 'mt-5'
              }`}
            >
              {group.qualities.map((quality) => (
                <Fact key={quality.quality} label={quality.quality} variant="figure" size={26}>
                  {formatCount(quality.members)}
                </Fact>
              ))}
            </div>
          )}
        </Section>

        <Section title="Présence dans les votes" className="max-md:band max-md:py-7">
          <Fact label="Votes portant une ligne pour ce groupe" variant="figure" size={40}>
            {formatCount(group.line_count)}
          </Fact>
          {group.first_scrutin_date && group.last_scrutin_date && (
            <Fact
              label="Du premier au dernier vote"
              variant="text"
              className="mt-4 border-t border-line pt-4"
            >
              {formatDate(group.first_scrutin_date)} → {formatDate(group.last_scrutin_date)}
            </Fact>
          )}
          {(group.reconstructed_count > 0 || group.silent_line_count > 0) && (
            <div className="mt-4 grid grid-cols-2 gap-x-5 gap-y-4 border-t border-line pt-4 md:grid-cols-1">
              {group.reconstructed_count > 0 && (
                <Fact label="Répartitions reconstituées" variant="figure" size={28}>
                  {formatCount(group.reconstructed_count)}
                </Fact>
              )}
              {group.silent_line_count > 0 && (
                <Fact label="Votes sans aucun membre du groupe" variant="figure" size={28}>
                  {formatCount(group.silent_line_count)}
                </Fact>
              )}
            </div>
          )}
        </Section>
      </div>

      <Section
        title="Participation"
        info={<GroupNotes group={group} />}
        className="mt-9 md:band md:mt-12 md:py-10"
      >
        {rates === null ? (
          <p className="text-sm text-ink-soft">
            L'Assemblée ne publie aucune position pour ce groupe : il n'y a
            aucun taux à calculer.
          </p>
        ) : (
          <>
            <RateBar rates={rates} />
            <div className="mt-3 md:mt-6 md:grid md:grid-cols-3 md:gap-x-8">
              <Rate
                label="Voix exprimées"
                swatch={RATE_BARS.expressed}
                perMille={rates.expressed_per_mille}
                count={counts.expressed}
                detail="pour et contre confondus"
              />
              <Rate
                label="Abstention"
                swatch={RATE_BARS.abstention}
                perMille={rates.abstention_per_mille}
                count={counts.abstentions}
              />
              <Rate
                label="Non-participation"
                swatch={RATE_BARS.absence}
                perMille={rates.absence_per_mille}
                count={counts.not_voting}
              />
            </div>
            <p className="mt-3 text-[13px] text-ink-faint md:mt-6 md:text-sm">
              Rapportés à {formatCount(counts.published_positions)} positions
              publiées, sur {formatCount(group.line_count)} vote
              {group.line_count > 1 ? 's' : ''}.
            </p>
          </>
        )}
      </Section>

      {/* Sans aucun scrutin, le groupe n'est pas au comparateur, qui répondrait 400. */}
      {group.line_count > 0 && (
        <Link
          to={`/votes-par-groupe?groupes=${encodeURIComponent(group.abbrev)}`}
          className="mt-8 flex min-h-14 items-center justify-between gap-4 rounded-2xl bg-accent px-[18px] py-2.5 text-[15px] font-[620] text-surface transition-colors hover:bg-accent-strong focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent md:max-w-md"
        >
          <span>Voir les votes de {group.abbrev}, vote par vote</span>
          <span>→</span>
        </Link>
      )}

      {/* Toutes les mises en garde dues (README.md §6) dans un seul encadré :
          cinq blocs empilés pesaient plus que la fiche qu'ils commentent. */}
      <Note desktopOnly flat className="mt-10">
        <GroupNotes
          group={group}
          className="md:columns-2 md:gap-x-12 [&>p]:mb-3 [&>p]:break-inside-avoid"
        />
      </Note>
    </div>
  )
}
