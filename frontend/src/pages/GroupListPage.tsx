import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
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
      className={`inline-block shrink-0 rounded-full ring-1 ring-inset ring-black/10 ${
        size === 'lg' ? 'h-3 w-3' : 'h-2.5 w-2.5'
      }`}
      style={{ backgroundColor: color ?? 'transparent' }}
      aria-hidden
    />
  )
}

function GroupRow({ group }: { group: GroupSummaryDto }) {
  return (
    <li>
      <Link
        to={`/groupes/${group.uid}`}
        className="group flex flex-col gap-1 px-4 py-3 transition-colors hover:bg-surface-soft sm:flex-row sm:items-center sm:justify-between sm:gap-6"
      >
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <GroupDot color={group.color} size="lg" />
            <span className="text-[15px] font-semibold text-ink transition-colors group-hover:text-accent">
              {group.abbrev}
            </span>
            <span className="text-sm text-ink-soft">{group.label}</span>
            {group.dissolved && <Pill>dissous</Pill>}
          </div>
          {group.former_abbrevs.length > 0 && (
            <p className="mt-0.5 text-xs text-ink-faint">
              anciennement {group.former_abbrevs.join(', ')}
            </p>
          )}
        </div>

        <p className="shrink-0 text-xs text-ink-faint sm:text-right">
          <span className="font-semibold text-ink-soft">
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
    <div className="space-y-4">
      <PageHeader
        title="Groupes parlementaires"
        lede={
          <>
            Les {data!.total} groupes de la {data!.groups[0]?.legislature}
            <sup>e</sup> législature. Ouvrez un groupe pour son effectif, ses
            dates et sa participation aux scrutins.
          </>
        }
      />

      <ul className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card">
        {data!.groups.map((group) => (
          <GroupRow key={group.uid} group={group} />
        ))}
      </ul>

      {/* Groupe ≠ parti, et les votes à main levée manquent : les deux lacunes
          sont dites, dans un seul encadré (README.md §3 et §2). */}
      <Note>
        <p>{data!.party_note}</p>
        <p className="mt-1">
          {data!.hand_vote_note}{' '}
          <Link to="/scrutins" className="text-accent underline">
            Voir tous les scrutins
          </Link>
        </p>
      </Note>
    </div>
  )
}
