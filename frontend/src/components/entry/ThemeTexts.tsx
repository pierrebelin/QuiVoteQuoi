import { useId, useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { fetchJson } from '../../lib/fetchJson'
import type { FamilyDto, TextListResponse } from '../../types/themes'
import TextList from '../TextList'
import { MethodNote } from '../ThemeBadges'
import { ErrorPanel, Loading } from '../ui'
import { FOCUS_RING } from './styles'

const PAGE_SIZE = 50

const PAGER_CLASS =
  'min-h-11 rounded-lg border border-line bg-surface px-3 text-sm font-medium text-ink-soft shadow-card transition-colors hover:bg-surface-soft hover:text-ink disabled:opacity-40'

/**
 * Tous les textes rattachés à un sujet, votés sur l'ensemble ou non.
 *
 * L'étape 3 ne montre que les votes sur l'ensemble : un texte examiné sans vote
 * final (amendements, motions, articles) n'y paraît pas. Cette liste, reprise de
 * l'ancienne fiche `/themes/:code`, garde chaque texte rattaché atteignable
 * depuis le parcours (README.md §2). Chargée à l'ouverture seulement.
 */
export default function ThemeTexts({ family }: { family: FamilyDto }) {
  const [open, setOpen] = useState(false)
  const [offset, setOffset] = useState(0)
  const panelId = useId()

  const texts = useQuery({
    queryKey: ['themes', family.code, offset],
    queryFn: () =>
      fetchJson<TextListResponse>(
        `/api/themes/${encodeURIComponent(family.code)}?limit=${PAGE_SIZE}&offset=${offset}`,
      ),
    enabled: open,
    placeholderData: keepPreviousData,
    retry: false,
  })

  const data = texts.data

  return (
    <div>
      <button
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpen((value) => !value)}
        className={`inline-flex min-h-11 items-center gap-1.5 rounded-md text-sm font-medium text-accent hover:underline ${FOCUS_RING}`}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={2.4}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
          className={`size-4 transition-transform ${open ? 'rotate-90' : ''}`}
        >
          <path d="M9 6l6 6-6 6" />
        </svg>
        {open ? 'Masquer les textes du sujet' : 'Tous les textes du sujet, votés ou non'}
      </button>

      {open && (
        <div id={panelId} className="mt-2 space-y-3">
          <p className="text-sm leading-relaxed text-ink-soft">{family.scope}</p>
          {texts.isError && <ErrorPanel error={texts.error} />}
          {!data && texts.isLoading && <Loading>Chargement des textes…</Loading>}
          {data && (
            <>
              <p className="text-xs text-ink-faint">
                {data.total} texte{data.total > 1 ? 's' : ''} rattaché{data.total > 1 ? 's' : ''}
              </p>
              <MethodNote note={data.method_note} />
              <TextList texts={data.items} />
              {data.total > PAGE_SIZE && (
                <div className="flex items-center justify-between gap-3">
                  <button
                    type="button"
                    disabled={offset === 0}
                    onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
                    className={`${PAGER_CLASS} ${FOCUS_RING}`}
                  >
                    Précédent
                  </button>
                  <span className="text-xs text-ink-faint tabular-nums">
                    {offset + 1} – {Math.min(offset + PAGE_SIZE, data.total)} sur {data.total}
                  </span>
                  <button
                    type="button"
                    disabled={offset + PAGE_SIZE >= data.total}
                    onClick={() => setOffset(offset + PAGE_SIZE)}
                    className={`${PAGER_CLASS} ${FOCUS_RING}`}
                  >
                    Suivant
                  </button>
                </div>
              )}
              <p className="text-xs text-ink-faint">
                Aucun texte n’est retiré du site faute de sujet :{' '}
                <Link to="/themes/non-rattaches" className="text-accent underline">
                  voir les textes sans sujet
                </Link>
                .
              </p>
            </>
          )}
        </div>
      )}
    </div>
  )
}
