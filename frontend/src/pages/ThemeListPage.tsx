import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { MethodNote } from '../components/ThemeBadges'
import { ErrorPanel, Loading, Note, PageHeader } from '../components/ui'
import type { FamiliesResponse, MethodResponse } from '../types/themes'

export default function ThemeListPage() {
  const families = useQuery({
    queryKey: ['themes'],
    queryFn: (): Promise<FamiliesResponse> =>
      fetch('/api/themes').then((res) => {
        if (!res.ok) throw new Error('Chargement des thèmes impossible')
        return res.json()
      }),
  })

  const method = useQuery({
    queryKey: ['themes', 'method'],
    queryFn: (): Promise<MethodResponse> =>
      fetch('/api/themes/method').then((res) => {
        if (!res.ok) throw new Error('Chargement de la méthode impossible')
        return res.json()
      }),
  })

  const coverage = new Map(
    method.data?.families.map((f) => [f.code, f]) ?? [],
  )

  if (families.isLoading) return <Loading>Chargement des thèmes…</Loading>
  if (families.error) return <ErrorPanel error={families.error} />

  return (
    <div className="space-y-4">
      <PageHeader
        title="Thèmes"
        lede={`Les votes de la législature, regroupés par famille thématique. Un texte peut appartenir à ${families.data?.max_families_per_text} familles au plus.`}
      />

      <MethodNote note={families.data!.method_note} />

      {/* Les familles tiennent en colonnes : seize entrées courtes
          empilées sur toute la largeur laissaient la page presque vide. */}
      <ul className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {families.data!.families.map((family) => {
          const stats = coverage.get(family.code)
          return (
            <li key={family.code}>
              <Link
                to={`/themes/${family.code}`}
                className="group flex h-full flex-col rounded-xl border border-line bg-surface px-4 py-3 shadow-card transition-shadow hover:shadow-card-hover"
              >
                <p className="text-[15px] font-semibold text-ink transition-colors group-hover:text-accent">
                  {family.label}
                </p>
                {stats && (
                  <p className="mt-1 text-xs text-ink-faint">
                    <span className="font-semibold text-ink-soft">
                      {stats.scrutin_count}
                    </span>{' '}
                    scrutin{stats.scrutin_count > 1 ? 's' : ''} ·{' '}
                    <span className="font-semibold text-ink-soft">
                      {stats.text_count}
                    </span>{' '}
                    texte{stats.text_count > 1 ? 's' : ''}
                  </p>
                )}
                <p className="mt-2 text-xs leading-relaxed text-ink-soft">
                  {family.scope}
                </p>
              </Link>
            </li>
          )
        })}
      </ul>

      {method.data && (
        <Note>
          <Link to="/themes/non-rattaches" className="text-accent underline">
            Textes non rattachés
          </Link>{' '}
          — {method.data.texts_total - method.data.texts_assigned} texte
          {method.data.texts_total - method.data.texts_assigned > 1 ? 's' : ''}{' '}
          sans famille, sur {method.data.texts_total}. Ils restent consultables :
          rien n'est retiré du site.
        </Note>
      )}
    </div>
  )
}
