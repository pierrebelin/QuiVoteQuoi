import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { BackLink } from '../components/ui'
import type {
  FamiliesResponse,
  TextListResponse,
  TextSummaryDto,
} from '../types/themes'
import { AdminTokenField } from '../components/AdminTokenField'
import { adminFetch } from '../lib/adminToken'

const PAGE_SIZE = 50

const FIELD =
  'mt-0.5 w-full rounded border border-line bg-canvas px-2 py-1 text-sm focus:border-accent focus:outline-none'

/**
 * CU-03 — Écran d'arbitrage. Accès par jeton partagé : ce n'est pas une page
 * publique, mais elle ne cache rien que le site n'affiche déjà.
 */
export default function ThemeArbitrationPage() {
  const queryClient = useQueryClient()
  const [source, setSource] = useState('unassigned')
  const [selected, setSelected] = useState<TextSummaryDto | null>(null)
  const [families, setFamilies] = useState<string[]>([])
  const [author, setAuthor] = useState('')
  const [motive, setMotive] = useState('')

  const referential = useQuery({
    queryKey: ['themes'],
    queryFn: (): Promise<FamiliesResponse> =>
      fetch('/api/themes').then((res) => res.json()),
  })

  const texts = useQuery({
    queryKey: ['arbitration', source],
    queryFn: (): Promise<TextListResponse> => {
      const url =
        source === 'unassigned'
          ? `/api/themes/unassigned?limit=${PAGE_SIZE}`
          : `/api/themes/${source}?limit=${PAGE_SIZE}`
      return fetch(url).then((res) => res.json())
    },
  })

  const arbitrate = useMutation({
    mutationFn: async () => {
      const response = await adminFetch('/api/themes/arbitrate', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          subject_kind: 'text',
          subject_id: selected!.key,
          families,
          author,
          motive: motive.trim() === '' ? null : motive,
        }),
      })
      return response.json()
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['arbitration'] })
      queryClient.invalidateQueries({ queryKey: ['themes'] })
      setSelected(null)
      setFamilies([])
      setMotive('')
    },
  })

  function select(text: TextSummaryDto) {
    setSelected(text)
    setFamilies(text.families.map((f) => f.code))
    setMotive('')
    arbitrate.reset()
  }

  const maxFamilies = referential.data?.max_families_per_text ?? 3
  const tooMany = families.length > maxFamilies
  const motiveRequired = families.length === 0 && motive.trim() === ''

  return (
    <div className="space-y-3">
      <div>
        <BackLink to="/par-sujet">Votes par sujet</BackLink>
        <h2 className="mt-1 text-2xl font-semibold tracking-tight">
          Arbitrage des rattachements
        </h2>
        <p className="mt-0.5 text-sm text-ink-soft">
          Retenir, corriger ou écarter les familles proposées. Chaque décision
          clôt le rattachement précédent et en ouvre un nouveau : l'historique
          est conservé.
        </p>
      </div>

      <div className="grid gap-3 rounded-lg border border-line bg-surface px-4 py-3 sm:grid-cols-3">
        <AdminTokenField />
        <label className="block text-sm">
          <span className="text-xs text-ink-faint">Auteur de la décision</span>
          <input
            value={author}
            onChange={(e) => setAuthor(e.target.value)}
            placeholder="votre nom"
            className={FIELD}
          />
        </label>
        <label className="block text-sm">
          <span className="text-xs text-ink-faint">Liste</span>
          <select
            value={source}
            onChange={(e) => {
              setSource(e.target.value)
              setSelected(null)
            }}
            className={FIELD}
          >
            <option value="unassigned">Textes non rattachés</option>
            {referential.data?.families.map((family) => (
              <option key={family.code} value={family.code}>
                {family.label}
              </option>
            ))}
          </select>
        </label>
      </div>

      {selected && (
        <div className="space-y-2 rounded-lg border border-accent/30 bg-accent-soft px-4 py-3">
          <div>
            <p className="text-sm font-medium">{selected.label}</p>
            <p className="text-xs text-ink-faint">
              {selected.scrutin_count} scrutin
              {selected.scrutin_count > 1 ? 's' : ''}
            </p>
          </div>

          <div className="flex flex-wrap gap-1.5">
            {referential.data?.families.map((family) => {
              const checked = families.includes(family.code)
              return (
                <label
                  key={family.code}
                  className={`cursor-pointer rounded border px-2 py-0.5 text-xs ${
                    checked
                      ? 'border-accent bg-surface text-accent font-medium'
                      : 'border-line bg-surface text-ink-soft'
                  }`}
                >
                  <input
                    type="checkbox"
                    className="mr-1.5"
                    checked={checked}
                    onChange={() =>
                      setFamilies(
                        checked
                          ? families.filter((c) => c !== family.code)
                          : [...families, family.code],
                      )
                    }
                  />
                  {family.label}
                </label>
              )
            })}
          </div>

          {tooMany && (
            <p className="text-xs text-abstain">{maxFamilies} familles au plus.</p>
          )}

          <label className="block text-sm">
            <span className="text-xs text-ink-faint">
              Motif {families.length === 0 ? '(obligatoire)' : '(facultatif)'}
            </span>
            <textarea
              value={motive}
              onChange={(e) => setMotive(e.target.value)}
              rows={2}
              className={`${FIELD} bg-surface`}
            />
          </label>

          <div className="flex items-center gap-3">
            <button
              type="button"
              disabled={
                arbitrate.isPending ||
                tooMany ||
                motiveRequired ||
                author.trim() === ''
              }
              onClick={() => arbitrate.mutate()}
              className="rounded bg-accent px-3 py-1 text-sm font-medium text-surface hover:bg-accent-strong disabled:opacity-40"
            >
              {arbitrate.isPending ? 'Enregistrement…' : 'Enregistrer'}
            </button>
            <button
              type="button"
              onClick={() => setSelected(null)}
              className="text-sm text-ink-soft hover:text-ink"
            >
              Annuler
            </button>
          </div>

          {arbitrate.error && (
            <p className="text-xs text-no">
              {(arbitrate.error as Error).message}
            </p>
          )}
        </div>
      )}

      {texts.isLoading ? (
        <p className="animate-pulse text-ink-faint">Chargement…</p>
      ) : (
        <ul className="divide-y divide-line rounded-lg border border-line bg-surface">
          {texts.data?.items.map((text) => (
            <li
              key={text.key}
              className="flex items-start justify-between gap-4 px-4 py-2"
            >
              <div className="min-w-0">
                <p className="text-sm">{text.label}</p>
                <p className="mt-0.5 text-xs text-ink-faint">
                  {text.scrutin_count} scrutin
                  {text.scrutin_count > 1 ? 's' : ''}
                  {text.families.length > 0
                    ? ` · ${text.families.map((f) => f.label).join(', ')}`
                    : ''}
                </p>
              </div>
              <button
                type="button"
                onClick={() => select(text)}
                className="shrink-0 rounded border border-line px-2 py-0.5 text-xs text-ink-soft hover:border-accent hover:text-accent"
              >
                Arbitrer
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
