import { Link } from 'react-router'
import type { ScrutinSummaryDto } from '../types/scrutins'
import { formatDateShort } from '../types/scrutins'
import { ListCard, Meta, Note, Pill, TallyLine, VoteBar } from './ui'

export { TallyLine } from './ui'

/**
 * Lacune de couverture affichée telle quelle : les votes à main levée sont
 * absents de la source, le site ne peut rien en dire.
 */
export function CoverageNote({ note }: { note: string }) {
  return <Note>{note}</Note>
}

/**
 * Ce que le scrutin porte, et sous quelle forme il a été mis aux voix.
 *
 * L'intitulé publié par l'Assemblée est procédural avant d'être informatif :
 * « l'ensemble de la proposition de loi visant à moderniser la gestion du
 * patrimoine immobilier de l'État (texte de la commission mixte paritaire) ».
 * Le serveur en tire un libellé court (« Vote sur l'article 3 ») qui sert de
 * titre ; l'intitulé officiel reste juste dessous, jamais retiré : c'est lui qui
 * fait foi, et lui seul distingue un vote sur l'ensemble d'un vote sur un
 * amendement du même texte.
 */
export function ScrutinRow({
  scrutin,
  showDossier = true,
}: {
  scrutin: ScrutinSummaryDto
  showDossier?: boolean
}) {
  return (
    <Link
      to={`/scrutins/${scrutin.uid}`}
      className="group flex flex-col gap-3 px-4 py-3 transition-colors hover:bg-surface-soft sm:flex-row sm:items-center sm:gap-6"
    >
      <div className="min-w-0 flex-1">
        <div className="mb-1 flex flex-wrap items-center gap-x-2 gap-y-1">
          {/* Le code — « adopté », « rejeté » — plutôt que le libellé, qui est
              une phrase entière (« L'Assemblée nationale a adopté… ») et ne
              tient pas dans une pastille. La phrase reste en infobulle. */}
          <Pill
            tone={scrutin.outcome_code === 'adopté' ? 'yes' : 'no'}
            title={scrutin.outcome_label}
          >
            {scrutin.outcome_code}
          </Pill>
          <Meta>
            n° {scrutin.number} · {formatDateShort(scrutin.date)}
          </Meta>
          {showDossier && scrutin.dossier_label && (
            <Meta>{scrutin.dossier_label}</Meta>
          )}
          {scrutin.has_reconstructed_tallies && (
            <Pill
              tone="info"
              title="La source ne publie pas les groupes sur ce scrutin : la répartition est reconstituée."
            >
              répartition reconstituée
            </Pill>
          )}
        </div>

        <p className="text-[15px] font-semibold leading-snug text-ink transition-colors group-hover:text-accent">
          {scrutin.short_label}
        </p>
        {/* L'objet officiel reste lisible sous le libellé court. Quand aucune
            forme ne reconnaît l'objet, le libellé court est l'objet tel quel :
            on ne le répète pas. */}
        {scrutin.short_label !== scrutin.subject && (
          <p className="mt-1 text-[13px] leading-snug text-ink-soft">
            {scrutin.subject}
          </p>
        )}
      </div>

      {/* Colonne de largeur fixe, pour que les barres s'alignent d'une ligne à
          l'autre. Chaque barre se normalise sur son propre scrutin : elle donne
          la répartition, pas le nombre de votants — deux scrutins n'ont pas le
          même nombre de présents et la longueur ne le dirait pas. */}
      <div className="shrink-0 space-y-1.5 sm:w-52">
        <VoteBar tally={scrutin.tally} />
        <TallyLine tally={scrutin.tally} dense />
      </div>
    </Link>
  )
}

export default function ScrutinList({
  scrutins,
  showDossier = true,
}: {
  scrutins: ScrutinSummaryDto[]
  showDossier?: boolean
}) {
  return (
    <ListCard>
      {scrutins.map((scrutin) => (
        <ScrutinRow key={scrutin.uid} scrutin={scrutin} showDossier={showDossier} />
      ))}
    </ListCard>
  )
}
