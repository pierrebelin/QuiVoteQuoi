import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { BackLink } from '../components/ui'
import { InfoTip } from '../components/InfoTip'
import TextList from '../components/TextList'
import { MethodNote, MethodText } from '../components/ThemeBadges'
import type { FamiliesResponse, TextListResponse } from '../types/themes'

const PAGE_SIZE = 50

export default function ThemeDetailPage() {
  const { code } = useParams<{ code: string }>()
  const [offset, setOffset] = useState(0)

  const families = useQuery({
    queryKey: ['themes'],
    queryFn: (): Promise<FamiliesResponse> =>
      fetch('/api/themes').then((res) => res.json()),
  })

  const texts = useQuery({
    queryKey: ['themes', code, offset],
    queryFn: (): Promise<TextListResponse> =>
      fetch(`/api/themes/${code}?limit=${PAGE_SIZE}&offset=${offset}`).then(
        (res) => {
          if (res.status === 404) throw new Error('Famille inconnue')
          if (!res.ok) throw new Error('Chargement impossible')
          return res.json()
        },
      ),
  })

  const family = families.data?.families.find((f) => f.code === code)

  if (texts.isLoading)
    return <p className="animate-pulse text-ink-faint">Chargement…</p>
  if (texts.error)
    return <p className="text-no">{(texts.error as Error).message}</p>

  const data = texts.data!

  return (
    <div className="space-y-3">
      <div>
        <BackLink to="/themes">Tous les thèmes</BackLink>
        <div className="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <h2 className="text-2xl font-semibold tracking-tight">
            {family?.label ?? code}
            <InfoTip label="À propos de ce thème" className="md:hidden">
              {family && <p>{family.scope}</p>}
              <p>
                <MethodText note={data.method_note} />
              </p>
            </InfoTip>
          </h2>
          <span className="text-xs text-ink-faint">
            {data.total} texte{data.total > 1 ? 's' : ''} rattaché
            {data.total > 1 ? 's' : ''}
          </span>
        </div>
        {family && <p className="mt-0.5 hidden text-sm text-ink-soft md:block">{family.scope}</p>}
      </div>

      <MethodNote note={data.method_note} />

      <TextList texts={data.items} />

      <div className="flex items-center justify-between">
        <button
          type="button"
          disabled={offset === 0}
          onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
          className="rounded-lg border border-line bg-surface px-3 py-1.5 text-sm font-medium text-ink-soft shadow-card transition-colors hover:bg-surface-soft hover:text-ink disabled:opacity-40"
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
          className="rounded-lg border border-line bg-surface px-3 py-1.5 text-sm font-medium text-ink-soft shadow-card transition-colors hover:bg-surface-soft hover:text-ink disabled:opacity-40"
        >
          Suivant
        </button>
      </div>

      <p className="text-xs text-ink-faint">
        <Link to="/themes/non-rattaches" className="text-accent underline">
          Voir les textes non rattachés
        </Link>
        <span className="hidden md:inline">
          {' '}
          — aucun texte n'est retiré du site faute de thème.
        </span>
        <InfoTip label="Textes non rattachés" className="md:hidden">
          Aucun texte n'est retiré du site faute de thème.
        </InfoTip>
      </p>
    </div>
  )
}
