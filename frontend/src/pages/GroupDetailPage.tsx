import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { InfoTip } from '../components/InfoTip'
import { BackLink, Card, ErrorPanel, Loading, Note, Pill } from '../components/ui'
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
 */
const RATE_BARS = {
  expressed: 'bg-sky-500',
  abstention: 'bg-amber-500',
  absence: 'bg-slate-400',
} as const

/** Les mises en garde dues (README.md §6), lues au même endroit quel que soit l'écran. */
function GroupNotes({ group }: { group: GroupDetailResponse }) {
  return (
    <div className="space-y-1">
      <p>{group.party_note}</p>
      <p>{group.rate_note}</p>
      <p>{group.comparison_note}</p>
      <p>{group.hand_vote_note}</p>
      {group.reconstructed_count > 0 && <p>{group.reconstructed_note}</p>}
      {group.silent_line_count > 0 && <p>{group.silent_line_note}</p>}
    </div>
  )
}

function Section({
  title,
  info,
  children,
}: {
  title: string
  info?: React.ReactNode
  children: React.ReactNode
}) {
  return (
    <Card className="px-4 py-3">
      <h3 className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
        {title}
        {info && (
          <InfoTip label={`Comment lire : ${title}`} className="md:hidden">
            {info}
          </InfoTip>
        )}
      </h3>
      <div className="mt-2">{children}</div>
    </Card>
  )
}

function Fact({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-0.5 border-b border-line py-1.5 last:border-0">
      <span className="text-xs text-ink-faint">{label}</span>
      <span className="text-sm font-medium">{children}</span>
    </div>
  )
}

/** Barre des trois parts. Elle rend visible ce que le pourcentage résume. */
function RateBar({ rates }: { rates: ParticipationRatesDto }) {
  return (
    <div className="flex h-2 w-full overflow-hidden rounded-full bg-surface-soft">
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
    <div className="border-b border-line py-2 last:border-0 md:border-0">
      <div className="flex items-baseline gap-2">
        <span
          className={`inline-block h-2 w-2 shrink-0 rounded-full ${swatch}`}
          aria-hidden
        />
        <span className="text-xs text-ink-soft">{label}</span>
        <span className="ml-auto figure text-[21px]">
          {formatPerMille(perMille)}
        </span>
      </div>
      <p className="pl-4 text-xs text-ink-faint">
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
    <div className="space-y-4">
      <div>
        <BackLink to="/groupes">Tous les groupes</BackLink>
        <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1">
          <GroupDot color={group.color} size="lg" />
          <h2 className="heading-2">
            {group.abbrev}
          </h2>
          <span className="text-sm text-ink-soft">{group.label}</span>
          {group.dissolved && <Pill>dissous</Pill>}
        </div>
      </div>

      {/* Les trois relevés factuels tiennent de front : la largeur du site les
          porte, et empilés ils poussaient la participation hors de l'écran. */}
      <div className="grid gap-3 md:grid-cols-3">
        <Section title="Identité">
          <Fact label="Législature">{group.legislature}<sup>e</sup></Fact>
          <Fact label="Constitué le">
            {group.created_on ? formatDate(group.created_on) : 'non publié'}
          </Fact>
          {group.dissolved_on && (
            <Fact label="Dissous le">{formatDate(group.dissolved_on)}</Fact>
          )}
          {group.former_abbrevs.length > 0 && (
            <Fact label="Sigles antérieurs">{group.former_abbrevs.join(', ')}</Fact>
          )}
        </Section>

        <Section title="Effectif">
          <Fact label={`Députés au ${formatDate(group.reference_date)}`}>
            {formatCount(group.member_count)}
          </Fact>
          <Fact label="Députés y ayant siégé depuis sa constitution">
            {formatCount(group.total_member_count)}
          </Fact>
          {group.published_member_range && (
            <Fact label="Effectif publié sur les votes">
              {group.published_member_range.stable
                ? formatCount(group.published_member_range.min)
                : `de ${formatCount(group.published_member_range.min)} à ${formatCount(
                    group.published_member_range.max,
                  )}`}
            </Fact>
          )}
          {group.qualities.map((quality) => (
            <Fact key={quality.quality} label={quality.quality}>
              {formatCount(quality.members)}
            </Fact>
          ))}
        </Section>

        <Section title="Présence dans les votes">
          <Fact label="Votes portant une ligne pour ce groupe">
            {formatCount(group.line_count)}
          </Fact>
          {group.first_scrutin_date && group.last_scrutin_date && (
            <Fact label="Du premier au dernier vote">
              {formatDate(group.first_scrutin_date)} → {formatDate(group.last_scrutin_date)}
            </Fact>
          )}
          {group.reconstructed_count > 0 && (
            <Fact label="Répartitions reconstituées">
              {formatCount(group.reconstructed_count)}
            </Fact>
          )}
          {group.silent_line_count > 0 && (
            <Fact label="Votes sans aucun membre du groupe">
              {formatCount(group.silent_line_count)}
            </Fact>
          )}
        </Section>
      </div>

      <Section title="Participation" info={<GroupNotes group={group} />}>
        {rates === null ? (
          <p className="text-sm text-ink-soft">
            L'Assemblée ne publie aucune position pour ce groupe : il n'y a
            aucun taux à calculer.
          </p>
        ) : (
          <>
            <RateBar rates={rates} />
            <div className="mt-2 grid gap-x-8 md:grid-cols-3">
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
            <p className="mt-2 text-xs text-ink-faint">
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
          className="flex items-center justify-between gap-4 rounded border border-line bg-surface px-4 py-3 shadow-card transition-shadow hover:shadow-card-hover"
        >
          <span className="text-sm font-semibold text-ink">
            Voir les votes de {group.abbrev}, vote par vote
          </span>
          <span className="text-sm text-accent">→</span>
        </Link>
      )}

      {/* Toutes les mises en garde dues (README.md §6) dans un seul encadré :
          cinq blocs empilés pesaient plus que la fiche qu'ils commentent. */}
      <Note desktopOnly>
        <GroupNotes group={group} />
      </Note>
    </div>
  )
}
