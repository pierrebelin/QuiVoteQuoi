import type { Bloc, PositionSchema } from '../../content/comprendre'
import PositionGlyph from '../entry/PositionGlyph'
import { RichText } from './RichText'

/*
 * Schémas de la page « Comprendre ». Aucun chiffre : ils montrent une
 * mécanique, jamais un vote réel (README.md §8). Le texte vient de
 * `content/comprendre.ts`, ces composants ne font que le dessiner.
 */

const PASTILLE: Record<Exclude<PositionSchema, 'non-votant'>, string> = {
  for: 'bg-yes-soft text-yes',
  against: 'bg-no-soft text-no',
  abstention: 'bg-abstain-soft text-abstain',
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

function Pastille({ position, taille = 'size-[26px]' }: { position: PositionSchema; taille?: string }) {
  if (position === 'non-votant') {
    return (
      <span
        aria-hidden
        className={`${taille} shrink-0 rounded-full border-2 border-dashed border-line-strong`}
      />
    )
  }
  return (
    <span
      aria-hidden
      className={`${taille} inline-flex shrink-0 items-center justify-center rounded-full ${PASTILLE[position]}`}
    >
      <PositionGlyph position={position} className="size-[55%]" />
    </span>
  )
}

/** Cadre commun des schémas : posé sur le fond de page, pour se détacher de la réponse. */
function Cadre({ titre, children }: { titre: string; children: React.ReactNode }) {
  return (
    <figure className="rounded border border-line bg-canvas p-3.5">
      <p className="mb-2.5 text-[13px] leading-[18px] font-semibold text-ink">{titre}</p>
      {children}
    </figure>
  )
}

function Positions({ bloc }: { bloc: Extract<Bloc, { kind: 'positions' }> }) {
  return (
    <ul>
      {bloc.items.map((item) => (
        <li key={item.position} className="flex items-start gap-2.5 border-t border-line py-2 first:border-t-0">
          <Pastille position={item.position} />
          <p className="flex-1 text-sm leading-relaxed text-ink-soft">
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
            className={`flex flex-col items-center gap-1.5 rounded-xl bg-surface px-1 py-2 text-center ${
              item.ensemble ? 'border-2 border-accent' : 'border border-line'
            }`}
          >
            <span
              className={`text-[11px] leading-[14px] ${
                item.ensemble ? 'font-semibold text-ink' : 'font-medium text-ink-soft'
              }`}
            >
              {item.libelle}
            </span>
            <Pastille position={item.position} taille="size-6" />
            <span className="sr-only">{MOT[item.position]}</span>
          </li>
        ))}
      </ol>
      <figcaption className="mt-2.5 text-[12.5px] leading-[18px] text-ink-soft">
        <RichText texte={bloc.legende} />
      </figcaption>
    </Cadre>
  )
}

function Majorite({ bloc }: { bloc: Extract<Bloc, { kind: 'majorite' }> }) {
  return (
    <Cadre titre={bloc.titre}>
      <div aria-hidden className="flex items-stretch gap-1.5">
        <div className="flex h-[26px] flex-[3] overflow-hidden rounded-lg">
          <span className="flex-1 bg-yes-bar" />
          <span className="flex-1 bg-no-bar" />
        </div>
        <div className="h-[26px] flex-1 rounded-lg border-2 border-dashed border-abstain-bar bg-abstain-soft" />
      </div>
      <div className="mt-1.5 flex gap-1.5 text-xs leading-4 text-ink-soft">
        <p className="flex-[3]">
          <RichText texte={bloc.exprimes} />
        </p>
        <p className="flex-1">{bloc.apart}</p>
      </div>
      <figcaption className="mt-1.5 text-[11.5px] leading-4 text-ink-faint">{bloc.mention}</figcaption>
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
                <span aria-hidden className="absolute top-5 -bottom-1.5 left-[7px] w-0.5 bg-line" />
              )}
              <span
                aria-hidden
                className={`relative mt-[3px] size-4 shrink-0 rounded-full border-2 ${
                  etape.surCeSite ? 'border-accent bg-accent' : 'border-line-strong bg-surface'
                }`}
              />
              <div className="flex-1">
                <p
                  className={`text-sm leading-5 font-semibold ${etape.surCeSite ? 'text-ink' : 'text-ink-soft'}`}
                >
                  {etape.titre}
                  {etape.surCeSite && (
                    <span className="ml-1.5 inline-block rounded-full bg-accent-soft px-[7px] align-middle text-[11px] leading-[18px] font-semibold text-accent">
                      votes sur ce site
                    </span>
                  )}
                </p>
                <p className="text-[12.5px] leading-[18px] text-ink-faint">{etape.precision}</p>
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
