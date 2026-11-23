import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { PageHeader } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import type { ExplanationCoverageDto } from '../types/finalVotes'

const COVERAGE_LABELS: [keyof ExplanationCoverageDto, string][] = [
  ['results_attached', 'Résultats rattachés à un vote'],
  ['results_unmatched', 'Résultats sans vote correspondant'],
  ['results_ambiguous', 'Résultats ambigus'],
  ['final_votes_unattached', 'Votes sur l’ensemble sans rattachement'],
  ['final_votes_without_block', 'Votes sur l’ensemble sans explication'],
  ['blocks_unattached', 'Explications sans vote rattaché'],
  ['headings_unrecognized', 'Intitulés non reconnus'],
  ['speakers_without_group', 'Orateurs sans groupe'],
]

/**
 * Méthode publique de la sélection des votes sur l'ensemble.
 *
 * Cette page sépare volontairement le vote final d'une lecture du devenir
 * final d'une loi : le premier est un fait publié par l'Assemblée, le second
 * exige des documents et des étapes que la liste de scrutins ne contient pas.
 */
export default function FinalVotesMethodPage() {
  const coverage = useQuery<ExplanationCoverageDto>({
    queryKey: ['votes-finaux-methode'],
    queryFn: () => fetchJson<ExplanationCoverageDto>('/api/votes-finaux/methode'),
  })

  return (
    <div className="space-y-9">
      <PageHeader
        title="Comment sont retenus les votes sur l’ensemble ?"
        lede="Cette méthode indique ce que signifie « vote final » dans les pages de comparaison. Elle ne permet pas de déduire, à elle seule, le sort ultérieur d’un texte."
      />

      <div className="grid gap-x-14 gap-y-9 lg:grid-cols-2">
        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">Un vote sur l’ensemble d’un texte</h3>
          <p className="mt-2 text-[15px] leading-[1.55] text-ink-soft">
            Nous retenons les votes dont l’objet officiel commence par « l’ensemble
            de… ». Ils portent sur la version complète du texte mise aux voix lors de
            cette lecture. Les votes sur un amendement, un article ou une motion restent
            consultables, mais ne sont pas confondus avec ce vote.
          </p>
        </section>

        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">« Final » ne veut pas dire « loi définitivement adoptée »</h3>
          <p className="mt-2 text-[15px] leading-[1.55] text-ink-soft">
            Un même texte peut être examiné à plusieurs lectures. Chaque lecture peut
            avoir son propre vote sur l’ensemble. Le résultat affiché dit uniquement si
            l’Assemblée a adopté ou rejeté la version soumise à ce vote, à cette date.
          </p>
        </section>

        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">Le lien avec un texte</h3>
          <p className="mt-2 text-[15px] leading-[1.55] text-ink-soft">
            Un vote est relié à un texte seulement lorsque la source officielle fournit
            ce rattachement. Le site ne rapproche jamais un vote et un texte parce que
            leurs titres se ressemblent. Lorsqu’aucun texte n’est fourni, le vote reste
            accessible depuis sa propre page, avec cette limite explicitement
            visible.
          </p>
        </section>

        <section className="border-t border-line pt-5">
          <h3 className="heading-5 text-ink">Le vote le plus récent d’un texte</h3>
          <p className="mt-2 text-[15px] leading-[1.55] text-ink-soft">
            Quand une future carte présentera un seul vote pour un texte, elle retiendra
            le vote sur l’ensemble le plus récent parmi ceux que la source rattache à ce
            texte. L’ordre est la date décroissante, puis le numéro de scrutin publié
            pour départager deux votes du même jour. Toutes les autres lectures restent
            accessibles dans la liste complète.
          </p>
        </section>
      </div>

      <section className="band py-8 md:py-10">
        <h3 className="heading-5 text-ink">Le rattachement des explications de vote</h3>
        <p className="mt-2 max-w-3xl text-[15px] leading-[1.55] text-ink-soft">
          Les explications de vote viennent des comptes rendus de séance. Une explication
          est rattachée à un vote sur l’ensemble par la date de séance et le résultat
          publié, jamais par ressemblance de titre. Les compteurs ci-dessous portent sur
          ce rattachement seulement : aucun ne se lit par groupe ni par député.
        </p>
        {coverage.data ? (
          <dl className="mt-4 grid gap-x-12 text-[15px] sm:grid-cols-2">
            {COVERAGE_LABELS.map(([key, label]) => (
              <div key={key} className="flex justify-between gap-3 border-t border-line py-2.5">
                <dt className="text-ink-soft">{label}</dt>
                <dd className="font-semibold tabular-nums">{coverage.data[key]}</dd>
              </div>
            ))}
          </dl>
        ) : null}
      </section>

      <section className="border-t border-line pt-5">
        <h3 className="heading-5 text-ink">Ce que cette sélection ne dit pas</h3>
        <p className="mt-2 max-w-3xl text-[15px] leading-[1.55] text-ink-soft">
          Elle ne résume pas encore le contenu juridique de la version votée, ne dit pas
          qu’un texte est devenu une loi, et ne mesure pas une « position » globale d’un
          groupe. La version exacte du texte et son résumé sourcé feront l’objet d’un
          référencement distinct avant leur affichage.
        </p>
        <p className="mt-4 text-[15px] text-ink-soft">
          <Link to="/votes-par-groupe" className="link">
            Retour aux votes des groupes
          </Link>
          {' · '}
          <Link to="/scrutins" className="link">
            Voir tous les votes
          </Link>
        </p>
      </section>
    </div>
  )
}
