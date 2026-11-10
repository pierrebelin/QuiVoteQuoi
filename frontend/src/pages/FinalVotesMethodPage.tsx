import { Link } from 'react-router'
import { Card, PageHeader } from '../components/ui'

/**
 * Méthode publique de la sélection des votes sur l'ensemble.
 *
 * Cette page sépare volontairement le vote final d'une lecture du devenir
 * final d'une loi : le premier est un fait publié par l'Assemblée, le second
 * exige des documents et des étapes que la liste de scrutins ne contient pas.
 */
export default function FinalVotesMethodPage() {
  return (
    <div className="space-y-5">
      <PageHeader
        title="Comment sont retenus les votes sur l’ensemble ?"
        lede="Cette méthode indique ce que signifie « vote final » dans les pages de comparaison. Elle ne permet pas de déduire, à elle seule, le sort ultérieur d’un texte."
      />

      <div className="grid gap-3 lg:grid-cols-2">
        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">Un vote sur l’ensemble d’un texte</h3>
          <p className="mt-2 text-sm leading-relaxed text-ink-soft">
            Nous retenons les votes dont l’objet officiel commence par « l’ensemble
            de… ». Ils portent sur la version complète du texte mise aux voix lors de
            cette lecture. Les votes sur un amendement, un article ou une motion restent
            consultables, mais ne sont pas confondus avec ce vote.
          </p>
        </Card>

        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">« Final » ne veut pas dire « loi définitivement adoptée »</h3>
          <p className="mt-2 text-sm leading-relaxed text-ink-soft">
            Un même texte peut être examiné à plusieurs lectures. Chaque lecture peut
            avoir son propre vote sur l’ensemble. Le résultat affiché dit uniquement si
            l’Assemblée a adopté ou rejeté la version soumise à ce vote, à cette date.
          </p>
        </Card>

        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">Le lien avec un texte</h3>
          <p className="mt-2 text-sm leading-relaxed text-ink-soft">
            Un vote est relié à un texte seulement lorsque la source officielle fournit
            ce rattachement. Le site ne rapproche jamais un vote et un texte parce que
            leurs titres se ressemblent. Lorsqu’aucun texte n’est fourni, le vote reste
            accessible depuis sa propre page, avec cette limite explicitement
            visible.
          </p>
        </Card>

        <Card className="px-4 py-3.5">
          <h3 className="text-sm font-semibold">Le vote le plus récent d’un texte</h3>
          <p className="mt-2 text-sm leading-relaxed text-ink-soft">
            Quand une future carte présentera un seul vote pour un texte, elle retiendra
            le vote sur l’ensemble le plus récent parmi ceux que la source rattache à ce
            texte. L’ordre est la date décroissante, puis le numéro de scrutin publié
            pour départager deux votes du même jour. Toutes les autres lectures restent
            accessibles dans la liste complète.
          </p>
        </Card>
      </div>

      <Card className="px-4 py-3.5">
        <h3 className="text-sm font-semibold">Ce que cette sélection ne dit pas</h3>
        <p className="mt-2 text-sm leading-relaxed text-ink-soft">
          Elle ne résume pas encore le contenu juridique de la version votée, ne dit pas
          qu’un texte est devenu une loi, et ne mesure pas une « position » globale d’un
          groupe. La version exacte du texte et son résumé sourcé feront l’objet d’un
          référencement distinct avant leur affichage.
        </p>
        <p className="mt-3 text-sm text-ink-soft">
          <Link to="/votes-par-groupe" className="text-accent underline underline-offset-2">
            Retour aux votes des groupes
          </Link>
          {' · '}
          <Link to="/scrutins" className="text-accent underline underline-offset-2">
            Voir tous les votes
          </Link>
        </p>
      </Card>
    </div>
  )
}
