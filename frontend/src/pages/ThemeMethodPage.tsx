import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import type { MethodResponse } from '../types/themes'

function Row({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="flex items-baseline justify-between gap-4 border-b border-line py-1">
      <span className="text-xs text-ink-soft">{label}</span>
      <span className="text-sm">{value}</span>
    </div>
  )
}

function Block({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <section className="rounded-lg border border-line bg-surface px-4 py-3">
      <h3 className="mb-1.5 text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
        {title}
      </h3>
      {children}
    </section>
  )
}

/** CU-06 — Méthode publiée (README.md §5, §9). */
export default function ThemeMethodPage() {
  const method = useQuery({
    queryKey: ['themes', 'method'],
    queryFn: (): Promise<MethodResponse> =>
      fetch('/api/themes/method').then((res) => {
        if (!res.ok) throw new Error('Chargement impossible')
        return res.json()
      }),
  })

  if (method.isLoading)
    return <p className="animate-pulse text-ink-faint">Chargement…</p>
  if (method.error)
    return <p className="text-no">{(method.error as Error).message}</p>

  const data = method.data!
  const linking = data.amendment_linking

  return (
    <div className="space-y-3">
      <div className="max-w-3xl">
        <Link to="/themes" className="text-sm text-accent hover:underline">
          ← Tous les thèmes
        </Link>
        <h2 className="mt-1 text-2xl font-semibold tracking-tight">
          Méthode de thématisation
        </h2>
        <p className="mt-1 text-sm text-ink-soft">{data.method_note}</p>
      </div>

      <div className="grid gap-3 md:grid-cols-2">
        <Block title="Ce qui porte le thème">
          <p className="text-sm text-ink-soft">{data.extraction_rule}</p>
          <p className="mt-1.5 text-sm text-ink-soft">
            Un scrutin reçoit les familles du texte qu'il met aux voix. Le lien
            entre un dossier législatif et un texte est établi par les scrutins
            que la source rattache aux deux — aucun rapprochement de libellés
            n'est deviné.
          </p>
        </Block>

        <Block title="Ce que fait le modèle">
          <p className="text-sm text-ink-soft">{data.model_scope}</p>
          <p className="mt-1.5 text-sm text-ink-soft">
            Un texte porte {data.max_families_per_text} familles au plus. Les
            familles « Société / libertés » et « Immigration » rattachent sur
            l'objet du texte, jamais sur son orientation.
          </p>
        </Block>

        <Block title="Ce que tranchent les règles">
          <p className="text-sm text-ink-soft">{data.rule_scope}</p>
          <ul className="mt-2 space-y-1.5">
            {data.rules.map((rule) => (
              <li key={rule.id} className="border-b border-line pb-1.5 last:border-0">
                <div className="flex flex-wrap items-baseline gap-x-1.5">
                  <code className="rounded bg-surface-soft px-1 py-0.5 text-[11px] text-ink-soft">
                    {rule.marker}
                  </code>
                  {rule.whole_word && (
                    <span className="text-xs text-ink-faint">(mot entier)</span>
                  )}
                  <span className="text-xs text-ink-faint">
                    → {rule.families.map((f) => f.label).join(', ')}
                  </span>
                </div>
                <p className="mt-0.5 text-xs text-ink-soft">{rule.statement}</p>
              </li>
            ))}
          </ul>
        </Block>

        <Block title="Couverture">
          <Row label="Textes débattus" value={data.texts_total} />
          <Row label="Textes rattachés à au moins une famille" value={data.texts_assigned} />
          <Row
            label="Textes soumis, aucune famille retenue"
            value={data.texts_without_family}
          />
          <Row
            label="Textes soumis, modèle sans réponse"
            value={data.texts_attempt_failed}
          />
          <Row label="Textes jamais soumis" value={data.texts_never_attempted} />
        </Block>

        <Block title="Scrutins">
          <Row label="Scrutins publiés par la source" value={data.scrutins_total} />
          <Row label="Scrutins rattachés à un texte" value={data.scrutins_with_text} />
          <Row
            label="Objets ne nommant aucun texte"
            value={data.scrutins_without_text}
          />
          <Row label="Scrutins portant au moins une famille" value={data.scrutins_assigned} />
          <p className="mt-1.5 text-xs text-ink-faint">
            Les scrutins sans texte et les textes sans famille restent
            consultables :{' '}
            <Link to="/themes/non-rattaches" className="text-accent underline">
              liste des non rattachés
            </Link>
            .
          </p>
        </Block>

        <Block title="Dossiers">
          <Row label="Dossiers ingérés" value={data.dossiers_total} />
          <Row label="Dossiers reliés à un texte voté" value={data.dossiers_linked_to_text} />
          <Row
            label="Dossiers sans scrutin, classés sur leur titre"
            value={data.dossiers_without_text}
          />
          <Row label="Dossiers portant une famille" value={data.dossiers_assigned} />
          <p className="mt-1.5 text-xs text-ink-faint">
            Un dossier relié à un texte voté reçoit les familles de ce texte : il
            n'est jamais soumis séparément. Seuls les dossiers qu'aucun scrutin ne
            relie à un texte sont classés sur leur propre titre.
          </p>
        </Block>

        <Block title="Rapprochement scrutins–amendements">
          <p className="mb-1.5 text-sm text-ink-soft">
            L'objet d'un scrutin cite des numéros d'amendement. Chaque numéro
            est cherché dans le dossier que la source rattache au scrutin : il
            est relié à l'amendement unique qui le porte, ou reste sans lien,
            avec sa cause.
          </p>
          <Row label="Numéros reliés à un amendement" value={linking.linked} />
          <Row
            label="dont départagés par l'article cité"
            value={linking.linked_by_cited_article}
          />
          {linking.absences.map((absence) => (
            <Row key={absence.code} label={absence.label} value={absence.count} />
          ))}
          <p className="mt-1.5 text-xs text-ink-faint">
            {data.amendment_linking_note}
          </p>
        </Block>
      </div>
    </div>
  )
}
