import { useQuery } from '@tanstack/react-query'
import Hemicycle from './Hemicycle'
import { PLAN_VUE } from './planHemicycle'
import { siegesDesGroupes } from './sieges'
import { fetchJson } from '../lib/fetchJson'
import type { TallyDto } from '../types/finalVotes'
import type { ScrutinDetailDto } from '../types/scrutins'

const COUNTS = [
  { key: 'votes_for', label: 'pour', className: 'text-yes' },
  { key: 'votes_against', label: 'contre', className: 'text-no' },
  { key: 'abstentions', label: 'abstentions', className: 'text-abstain' },
] as const

/**
 * Hémicycle du dernier vote sur l'ensemble, pour la carte large de l'accueil :
 * les sièges du scrutin, puis le décompte de toute l'Assemblée en grands
 * chiffres. Les nombres viennent du décompte publié, jamais des sièges
 * dessinés : un siège sans position localisable ne doit rien retrancher.
 */
export default function LatestVoteHemicycle({
  scrutinUid,
  tally,
  labelledBy,
}: {
  scrutinUid: string
  tally: TallyDto
  labelledBy?: string
}) {
  const scrutin = useQuery({
    queryKey: ['scrutin', scrutinUid],
    queryFn: () => fetchJson<ScrutinDetailDto>(`/api/scrutins/${scrutinUid}`),
    retry: false,
  })
  const seats = scrutin.data ? siegesDesGroupes(scrutin.data.groups) : []

  return (
    <figure className="mx-auto w-full max-w-[440px]">
      {scrutin.data && seats.length > 0 ? (
        <Hemicycle votes={seats} labelledBy={labelledBy} />
      ) : scrutin.data || scrutin.isError ? (
        <p className="rounded border border-line bg-surface-soft px-3 py-2 text-sm text-ink-soft">
          {scrutin.isError
            ? 'Les sièges de ce vote ne sont pas disponibles pour le moment.'
            : 'La source ne publie aucune position nominale exploitable pour représenter l’hémicycle de ce vote.'}
        </p>
      ) : (
        // Place réservée pendant le chargement : la carte ne saute pas.
        <div
          aria-hidden
          className="w-full"
          style={{ aspectRatio: `${PLAN_VUE.largeur} / ${PLAN_VUE.hauteur}` }}
        />
      )}
      <figcaption className="mt-3.5 grid grid-cols-3 gap-2">
        {COUNTS.map((count) => (
          <span key={count.key}>
            <span
              className={`block font-heading text-[40px] leading-none font-[560] tracking-[-0.04em] ${count.className}`}
            >
              {tally[count.key]}
            </span>
            <span className="mt-1 block text-[13px] leading-[18px] text-ink-soft">
              {count.label}
            </span>
          </span>
        ))}
      </figcaption>
    </figure>
  )
}
