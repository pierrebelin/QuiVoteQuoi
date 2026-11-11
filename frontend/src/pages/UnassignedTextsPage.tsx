import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { BackLink } from '../components/ui'
import TextList from '../components/TextList'
import type { TextListResponse } from '../types/themes'

const PAGE_SIZE = 50

/** CU-05 — Un objet non rattaché reste consultable (RM-01). */
export default function UnassignedTextsPage() {
  const [offset, setOffset] = useState(0)

  const texts = useQuery({
    queryKey: ['themes', 'unassigned', offset],
    queryFn: (): Promise<TextListResponse> =>
      fetch(`/api/themes/unassigned?limit=${PAGE_SIZE}&offset=${offset}`).then(
        (res) => {
          if (!res.ok) throw new Error('Chargement impossible')
          return res.json()
        },
      ),
  })

  if (texts.isLoading)
    return <p className="animate-pulse text-ink-faint">Chargement…</p>
  if (texts.error)
    return <p className="text-no">{(texts.error as Error).message}</p>

  const data = texts.data!

  return (
    <div className="space-y-3">
      <div>
        <BackLink to="/par-sujet">Votes par sujet</BackLink>
        <div className="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <h2 className="heading-2">
            Textes non rattachés
          </h2>
          <span className="text-xs text-ink-faint">
            {data.total} texte{data.total > 1 ? 's' : ''}
          </span>
        </div>
        <p className="mt-0.5 text-sm text-ink-soft">
          Ces textes n'ont aucune famille. Ils restent consultables et leurs
          votes sont exposés comme les autres : le site ne retire rien faute de
          sujet.
        </p>
      </div>

      <TextList texts={data.items} showFamilies={false} />

      <div className="flex items-center justify-between">
        <button
          type="button"
          disabled={offset === 0}
          onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
          className="rounded border border-line bg-surface px-3 py-1.5 text-sm font-medium text-ink-soft shadow-card transition-colors hover:bg-surface-soft hover:text-ink disabled:opacity-40"
        >
          Précédent
        </button>
        <span className="text-xs text-ink-faint">
          {data.total === 0
            ? '0'
            : `${offset + 1} – ${Math.min(offset + PAGE_SIZE, data.total)} sur ${data.total}`}
        </span>
        <button
          type="button"
          disabled={offset + PAGE_SIZE >= data.total}
          onClick={() => setOffset(offset + PAGE_SIZE)}
          className="rounded border border-line bg-surface px-3 py-1.5 text-sm font-medium text-ink-soft shadow-card transition-colors hover:bg-surface-soft hover:text-ink disabled:opacity-40"
        >
          Suivant
        </button>
      </div>
    </div>
  )
}
