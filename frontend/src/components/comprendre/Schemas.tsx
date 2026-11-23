import type { Bloc, PositionSchema } from '../../content/comprendre'
import PositionGlyph from '../entry/PositionGlyph'
import { SWATCH_CLASS } from '../entry/styles'
import { RichText } from './RichText'

/*
 * Schémas de la page « Comprendre ». Aucun chiffre : ils montrent une
 * mécanique, jamais un vote réel (README.md §8). Le texte vient de
 * `content/comprendre.ts`, ces composants ne font que le dessiner.
 */

/** Disque plein de la refonte A : le même que la pastille d'état du reste du site. */
const PASTILLE: Record<Exclude<PositionSchema, 'non-votant'>, string> = {
  for: SWATCH_CLASS.for,
  against: SWATCH_CLASS.against,
  abstention: SWATCH_CLASS.abstention,
}

const MOT: Record<PositionSchema, string> = {
  for: 'pour',
  against: 'contre',
  abstention: 'abstention',
  'non-votant': 'non-votant',
}

const LIBELLE: Record<PositionSchema, string> = {
  for: 'text-yes',
  against: 'text-no',
  abstention: 'text-abstain',
  'non-votant': 'text-ink',
}

function Pastille({ position, taille = 'size-5' }: { position: PositionSchema; taille?: string }) {
  if (position === 'non-votant') {
    return (
      <span
        aria-hidden
        className={`${taille} shrink-0 rounded-full border-[1.5px] border-dashed border-ink-faint`}
      />
    )
  }
  return (
    <span
      aria-hidden
      className={`${taille} inline-flex shrink-0 items-center justify-center rounded-full ${PASTILLE[position]}`}
    >
      <PositionGlyph position={position} className="size-[62%]" />
    </span>
  )
}

/** Cadre commun des schémas : une carte grise sans bordure, ses éléments en blanc dessus. */
function Cadre({ titre, children }: { titre: string; children: React.ReactNode }) {
  return (
    <figure className="rounded-2xl bg-surface p-3.5">
      <p className="mb-3 text-sm leading-[18px] font-semibold text-ink">{titre}</p>
      {children}
    </figure>
  )
}

function Positions({ bloc }: { bloc: Extract<Bloc, { kind: 'positions' }> }) {
  return (
    <ul>
      {bloc.items.map((item) => (
        <li key={item.position} className="flex items-start gap-2.5 border-t border-line py-2.5 first:border-t-0">
          <span className="flex pt-px">
            <Pastille position={item.position} />
          </span>
          <p className="flex-1 text-[15px] leading-[1.45] text-ink-soft">
            <strong className={`font-semibold ${LIBELLE[item.position]}`}>{item.libelle}</strong> — {item.texte}
          </p>
        </li>
      ))}
    </ul>
  )
}

function TexteEnVotes({ bloc }: { bloc: Extract<Bloc, { kind: 'texte-en-votes' }> }) {
  return (
    <Cadre titre={bloc.titre}>
      <ol className="grid grid-cols-4 gap-1.5">
        {bloc.cases.map((item, index) => (
          <li
            key={index}
            className={`flex min-h-[72px] flex-col items-center justify-between gap-2 rounded-xl bg-canvas px-1 py-2 text-center ${
              item.ensemble ? 'ring-2 ring-accent ring-inset' : ''
            }`}
          >
            <span
              className={`text-xs leading-[15px] ${
                item.ensemble ? 'font-semibold text-ink' : 'font-medium text-ink-soft'
              }`}
            >
              {item.libelle}
            </span>
            <Pastille position={item.position} taille="size-[18px]" />
            <span className="sr-only">{MOT[item.position]}</span>
          </li>
        ))}
      </ol>
      <figcaption className="mt-3 text-[13px] leading-[18px] text-ink-soft">
        <RichText texte={bloc.legende} />
      </figcaption>
    </Cadre>
  )
}

function Majorite({ bloc }: { bloc: Extract<Bloc, { kind: 'majorite' }> }) {
  return (
    <Cadre titre={bloc.titre}>
      <div aria-hidden className="flex items-stretch gap-1.5">
        <div className="flex h-6 flex-[3] gap-0.5 overflow-hidden rounded-md">
          <span className="flex-1 bg-yes-bar" />
          <span className="flex-1 bg-no-bar" />
        </div>
        <div className="h-6 flex-1 rounded-md border-[1.5px] border-dashed border-abstain-bar bg-abstain-soft" />
      </div>
      <div className="mt-2 flex gap-1.5 text-[13px] leading-[18px] text-ink-soft">
        <p className="flex-[3]">
          <RichText texte={bloc.exprimes} />
        </p>
        <p className="flex-1">{bloc.apart}</p>
      </div>
      <figcaption className="mt-2 text-[13px] leading-4 text-ink-faint">{bloc.mention}</figcaption>
    </Cadre>
  )
}

function Parcours({ bloc }: { bloc: Extract<Bloc, { kind: 'parcours' }> }) {
  return (
    <Cadre titre={bloc.titre}>
      <ol>
        {bloc.etapes.map((etape, index) => {
          const derniere = index === bloc.etapes.length - 1
          return (
            <li key={etape.titre} className={`relative flex gap-3 ${derniere ? '' : 'pb-3.5'}`}>
              {!derniere && (
                <span aria-hidden className="absolute top-5 -bottom-1.5 left-[7px] w-0.5 bg-line-strong" />
              )}
              <span
                aria-hidden
                className={`relative mt-[3px] size-4 shrink-0 rounded-full border-2 ${
                  etape.surCeSite ? 'border-ink bg-ink' : 'border-line-strong bg-canvas'
                }`}
              />
              <div className="flex-1">
                <p
                  className={`text-[15px] leading-5 font-semibold ${etape.surCeSite ? 'text-ink' : 'text-ink-soft'}`}
                >
                  {etape.titre}
                  {etape.surCeSite && (
                    <span className="ml-1.5 inline-block rounded-full bg-accent-soft px-2 align-middle text-xs leading-[22px] font-semibold text-accent-ink">
                      votes sur ce site
                    </span>
                  )}
                </p>
                <p className="text-[13px] leading-[18px] text-ink-faint">{etape.precision}</p>
              </div>
            </li>
          )
        })}
      </ol>
    </Cadre>
  )
}

/** Blocs de réponse qui ne sont pas de la prose. Renvoie `null` pour les autres. */
export function SchemaView({ bloc }: { bloc: Bloc }) {
  switch (bloc.kind) {
    case 'positions':
      return <Positions bloc={bloc} />
    case 'texte-en-votes':
      return <TexteEnVotes bloc={bloc} />
    case 'majorite':
      return <Majorite bloc={bloc} />
    case 'parcours':
      return <Parcours bloc={bloc} />
    default:
      return null
  }
}
