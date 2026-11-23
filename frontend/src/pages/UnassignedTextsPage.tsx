import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { BackLink, Button } from '../components/ui'
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
    <div className="space-y-6">
      <div>
        <BackLink to="/par-sujet">Votes par sujet</BackLink>
        <div className="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <h2 className="heading-2 md:heading-1">
            Textes non rattachés
          </h2>
          <span className="text-[13px] text-ink-faint">
            {data.total} texte{data.total > 1 ? 's' : ''}
          </span>
        </div>
        <p className="mt-2 max-w-2xl text-base leading-normal text-ink-soft md:mt-3 md:text-lg md:leading-relaxed">
          Ces textes n'ont aucune famille. Ils restent consultables et leurs
          votes sont exposés comme les autres : le site ne retire rien faute de
          sujet.
        </p>
      </div>

      <TextList texts={data.items} showFamilies={false} />

      <div className="flex items-center justify-between">
        <Button disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}>
          Précédent
        </Button>
        <span className="text-[13px] text-ink-faint">
          {data.total === 0
            ? '0'
            : `${offset + 1} – ${Math.min(offset + PAGE_SIZE, data.total)} sur ${data.total}`}
        </span>
        <Button disabled={offset + PAGE_SIZE >= data.total} onClick={() => setOffset(offset + PAGE_SIZE)}>
          Suivant
        </Button>
      </div>
    </div>
  )
}
