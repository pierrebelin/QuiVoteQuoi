import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { Icon } from '../components/icons'
import { formatDate } from '../types/scrutins'
import {
  ErrorPanel,
  Loading,
  Note,
  PageHeader,
  Pill,
} from '../components/ui'
import { formatCount, type GroupListResponse, type GroupSummaryDto } from '../types/groups'

/** Pastille d'identité du groupe. La couleur est celle que l'Assemblée publie. */
export function GroupDot({
  color,
  size = 'sm',
}: {
  color: string | null
  size?: 'sm' | 'lg'
}) {
  return (
    <span
      className={`inline-block shrink-0 rounded-full ring-1 ring-inset ring-chip-edge ${
        size === 'lg' ? 'h-3 w-3' : 'h-2.5 w-2.5'
      }`}
      style={{ backgroundColor: color ?? 'transparent' }}
      aria-hidden
    />
  )
}

/**
 * Ligne de la liste (refonte A) : à plat, un filet entre deux groupes. Sur
 * téléphone, l'effectif passe sous l'intitulé, le chiffre en Bricolage ; à
 * partir de 768 px, il se range à droite, comme avant.
 */
function GroupRow({ group }: { group: GroupSummaryDto }) {
  return (
    <li>
      <Link
        to={`/groupes/${group.uid}`}
        className="group grid grid-cols-[12px_minmax(0,1fr)_16px] items-start gap-x-3 py-3.5 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent md:grid-cols-[12px_minmax(0,1fr)_auto_16px] md:items-center md:gap-x-6 md:py-4"
      >
        <span className="row-span-2 pt-[7px] md:row-span-1 md:pt-0">
          <GroupDot color={group.color} size="lg" />
        </span>
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-x-2.5 gap-y-0.5">
            <span className="order-1 text-base font-bold tracking-[-0.01em] text-ink transition-colors group-hover:text-accent">
              {group.abbrev}
            </span>
            <span className="order-3 basis-full text-[15px] leading-snug text-ink-soft md:order-2 md:basis-auto">
              {group.label}
            </span>
            {group.dissolved && (
              <span className="order-2 md:order-3">
                <Pill>dissous</Pill>
              </span>
            )}
          </div>
          {group.former_abbrevs.length > 0 && (
            <p className="mt-0.5 text-[13px] text-ink-faint">
              anciennement {group.former_abbrevs.join(', ')}
            </p>
          )}
        </div>

        <p
          className={`col-start-2 row-start-2 mt-1.5 text-[13px] leading-[1.45] text-ink-faint md:col-start-3 md:row-start-1 md:mt-0 md:text-right ${
            group.dissolved ? '[&>.figure]:text-ink-faint' : ''
          }`}
        >
          <span className="figure mr-0.5 text-[22px] text-ink md:text-2xl">
            {formatCount(group.member_count)}
          </span>{' '}
          député{group.member_count > 1 ? 's' : ''} au{' '}
          {formatDate(group.reference_date)}
          {group.created_on && (
            <span className="block">
              constitué le {formatDate(group.created_on)}
            </span>
          )}
          {group.dissolved_on && (
            <span className="block">
              dissous le {formatDate(group.dissolved_on)}
            </span>
          )}
        </p>

        <Icon
          name="chevron"
          className="col-start-3 row-span-2 row-start-1 size-4 self-center text-ink-faint md:col-start-4 md:row-span-1"
        />
      </Link>
    </li>
  )
}

/**
 * Liste des groupes parlementaires de la législature.
 *
 * Aucun taux ici, volontairement. Treize taux d'abstention alignés en colonnes
 * forment un tableau qui se lit comme un classement — ce que README.md §6
 * interdit — alors que les mêmes chiffres sur la fiche d'un seul groupe restent
 * une description. Les taux sont donc sur la fiche, une page par groupe.
 */
export default function GroupListPage() {
  const { data, isLoading, isError, error } = useQuery<GroupListResponse>({
    queryKey: ['groupes'],
    queryFn: () =>
      fetch('/api/groupes').then(async (res) => {
        if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`)
        return res.json()
      }),
  })

  if (isLoading) return <Loading>Chargement des groupes…</Loading>
  if (isError) return <ErrorPanel error={error} />

  return (
    <div>
      <PageHeader
        info={
          <>
            <p>{data!.party_note}</p>
            <p>
              {data!.hand_vote_note}{' '}
              <Link to="/scrutins" className="link">
                Voir tous les votes
              </Link>
            </p>
          </>
        }
        title="Groupes parlementaires"
        lede={
          <>
            Les {data!.total} groupes de la {data!.groups[0]?.legislature}
            <sup>e</sup> législature. Ouvrez un groupe pour son effectif, ses
            dates et sa participation aux votes.
          </>
        }
      />

      <ul className="divide-y divide-line border-y border-line">
        {data!.groups.map((group) => (
          <GroupRow key={group.uid} group={group} />
        ))}
      </ul>

      {/* Groupe ≠ parti, et les votes à main levée manquent : les deux lacunes
          sont dites, dans un seul encadré (README.md §3 et §2). */}
      <Note desktopOnly flat className="mt-6 max-w-2xl">
        <p>{data!.party_note}</p>
        <p className="mt-3">
          {data!.hand_vote_note}{' '}
          <Link to="/scrutins" className="link">
            Voir tous les votes
          </Link>
        </p>
      </Note>
    </div>
  )
}
