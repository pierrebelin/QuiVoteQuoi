import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { FamilyIllustration } from '../components/FamilyIllustration'
import { Icon } from '../components/icons'
import { InfoTip } from '../components/InfoTip'
import { MethodNote, MethodText } from '../components/ThemeBadges'
import { ErrorPanel, Loading, Note, PageHeader } from '../components/ui'
import { firstSentence } from '../lib/text'
import { formatCount } from '../types/groups'
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
        info={
          <p>
            <MethodText note={families.data!.method_note} />
          </p>
        }
        title="Thèmes"
        lede={`Les votes de la législature, regroupés par famille thématique. Un texte peut appartenir à ${families.data?.max_families_per_text} familles au plus.`}
      />

      <MethodNote note={families.data!.method_note} />

      {/* Sous 640 px : une ligne aérée par thème, vignette de 56 px, première
          phrase du périmètre et décomptes ; toute la ligne est le lien. */}
      <ul className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card sm:hidden">
        {families.data!.families.map((family) => {
          const stats = coverage.get(family.code)
          return (
            <li key={family.code}>
              <Link
                to={`/themes/${family.code}`}
                className="group grid grid-cols-[56px_minmax(0,1fr)_16px] items-center gap-3 px-3.5 py-3 transition-colors hover:bg-surface-soft focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent"
              >
                <FamilyIllustration
                  code={family.code}
                  className="size-14 rounded-[14px]"
                  glyphClassName="size-[30px]"
                />
                <div className="min-w-0">
                  <p className="text-[15.5px] leading-tight font-semibold text-ink">
                    {family.label}
                  </p>
                  <p className="mt-0.5 line-clamp-2 text-[13px] leading-[1.35] text-ink-soft">
                    {firstSentence(family.scope)}
                  </p>
                  {stats && (
                    <p className="mt-0.5 text-xs text-ink-soft">
                      {formatCount(stats.scrutin_count)} scrutin
                      {stats.scrutin_count > 1 ? 's' : ''} ·{' '}
                      {formatCount(stats.text_count)} texte
                      {stats.text_count > 1 ? 's' : ''}
                    </p>
                  )}
                </div>
                <Icon name="chevron" className="size-4 text-ink-faint" />
              </Link>
            </li>
          )
        })}
      </ul>

      {/* À partir de 640 px : une vignette et un titre par famille ; le
          périmètre s'affiche au survol (ou au focus clavier) par-dessus la
          vignette. Sur un écran tactile, sans survol, il reste écrit sous le
          titre. */}
      <ul className="hidden grid-cols-3 gap-2.5 sm:grid lg:grid-cols-4">
        {families.data!.families.map((family) => {
          const stats = coverage.get(family.code)
          return (
            <li key={family.code}>
              <Link
                to={`/themes/${family.code}`}
                className="group flex h-full flex-col overflow-hidden rounded-xl border border-line bg-surface shadow-card transition-shadow hover:shadow-card-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
              >
                <div className="relative aspect-[2/1]">
                  <FamilyIllustration code={family.code} className="size-full" />
                  <p className="absolute inset-0 hidden items-center-safe overflow-y-auto bg-surface/95 px-3 py-2 text-[11px] leading-snug text-ink-soft opacity-0 backdrop-blur-sm transition-opacity duration-200 group-hover:opacity-100 group-focus-visible:opacity-100 [@media(hover:hover)]:flex">
                    {family.scope}
                  </p>
                </div>
                <div className="flex flex-1 flex-col border-t border-line px-3 py-2">
                  <p className="text-[13px] font-semibold leading-snug text-ink transition-colors group-hover:text-accent">
                    {family.label}
                  </p>
                  {stats && (
                    <p className="mt-0.5 text-[11px] text-ink-faint">
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
                  <p className="mt-1.5 line-clamp-3 text-xs leading-relaxed text-ink-soft [@media(hover:hover)]:hidden">
                    {family.scope}
                  </p>
                </div>
              </Link>
            </li>
          )
        })}
      </ul>

      {method.data && (
        <Note desktopOnly>
          <Link to="/themes/non-rattaches" className="text-accent underline">
            Textes non rattachés
          </Link>{' '}
          — {method.data.texts_total - method.data.texts_assigned} texte
          {method.data.texts_total - method.data.texts_assigned > 1 ? 's' : ''}{' '}
          sans famille, sur {method.data.texts_total}. Ils restent consultables :
          rien n'est retiré du site.
        </Note>
      )}
      {method.data && (
        <p className="flex items-center text-xs md:hidden">
          <Link to="/themes/non-rattaches" className="text-accent underline">
            Textes non rattachés
          </Link>
          <InfoTip label="Textes non rattachés">
            {method.data.texts_total - method.data.texts_assigned} texte
            {method.data.texts_total - method.data.texts_assigned > 1 ? 's' : ''}{' '}
            sans famille, sur {method.data.texts_total}. Ils restent consultables :
            rien n'est retiré du site.
          </InfoTip>
        </p>
      )}
    </div>
  )
}
