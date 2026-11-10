import type { ReactNode } from 'react'
import { Link } from 'react-router'
import { InfoTip } from '../components/InfoTip'
import { Card, Note } from '../components/ui'
import { ALL_LOGOS } from '../content/logos'
import { PORTRAITS } from '../content/portraits'
import { GITHUB_REPOSITORY_URL } from '../lib/repository'

/**
 * Ce qui tient la neutralité : trois contraintes vérifiables sur la page, pas
 * une déclaration d’intention. La deuxième est signalée comme le seul endroit
 * où un jugement entre dans le produit (README.md §5).
 */
const PRINCIPE = [
  {
    titre: 'Tout est ingéré',
    texte:
      'Tous les scrutins publiés par l’Assemblée entrent dans la base, sans sélection. Le tri ordonne l’affichage, il ne filtre jamais. Une lacune connue est affichée comme telle.',
    jugement: false,
  },
  {
    titre: 'Les textes sont rattachés à des sujets',
    texte:
      'La méthode est publiée, chaque rattachement est révisable et son historique est conservé.',
    jugement: true,
  },
  {
    titre: 'Les chiffres sont affichés bruts',
    texte:
      'Des nombres, pas d’adverbes. La position de chaque député est consultable au détail du scrutin, et chaque page renvoie à la source officielle.',
    jugement: false,
  },
]

/**
 * Ce que le site refuse de faire, énoncé à l’accueil plutôt qu’enfoui dans une
 * page annexe : c’est la contrepartie qui rend la promesse vérifiable
 * (README.md §6).
 */
const ANTI_PROMESSES = [
  'Aucune note, aucun classement, aucun score attribué à un groupe ou à une personne.',
  'Aucun score de cohérence : un écart entre un discours et un vote se constate, il ne se calcule pas ici.',
  'Aucune évaluation d’un texte : ni bon, ni mauvais, ni efficace.',
  'Aucune traduction d’un groupe parlementaire en parti politique.',
  'Aucun chiffre produit par un modèle de langage.',
  'Aucune prédiction électorale, aucune comparaison de programmes.',
]

/**
 * Les pièces qui documentent un vote. Le site ne rédige pas les motivations des
 * députés — il rassemble ce qui est attribué et publié, et le lecteur conclut
 * (README.md §6).
 */
const PIECES = [
  {
    titre: 'L’objet exact mis aux voix',
    texte:
      'Un scrutin porte souvent sur un amendement, un article ou une motion, pas sur l’ensemble du texte. L’intitulé officiel est affiché entier : c’est lui qui dit sur quoi l’accord ou le refus portait.',
  },
  {
    titre: 'Le dossier législatif',
    texte:
      'Qui a déposé le texte et avec quel exposé des motifs, son passage en commission puis en séance, et son sort : promulgation, rejet, ou dossier sans acte de clôture.',
  },
  {
    titre: 'La position de chaque député',
    texte:
      'Nom par nom, siège par siège. C’est là que se lisent les fractures internes qu’un total de groupe efface — et là que la consigne se distingue du vote réel.',
  },
  {
    titre: 'Les mises au point',
    texte:
      'Les déclarations postérieures de députés indiquant avoir voulu voter autrement. Elles sont affichées à part et ne modifient aucun décompte.',
  },
]

/**
 * Section de l'accueil.
 *
 * Sans marqueur, six blocs de cartes empilés sur le même fond forment un seul
 * long texte : le lecteur ne sait plus où un propos s'arrête. Le filet en tête
 * de section, l'amorce colorée qui le mord, puis le surtitre et le titre
 * redonnent la scansion — c'est le seul endroit du site qui en a besoin,
 * l'accueil étant la seule page qui enchaîne des propos différents plutôt
 * qu'une liste homogène.
 *
 * La couleur ne vient que du rôle « accent », celui des liens : les teintes de
 * position — vert, rouge, ambre — restent réservées aux votes, et une section
 * décorée avec elles se lirait comme une prise de parti (README.md §6).
 *
 * `panneau` pose la section sur un fond légèrement enfoncé, au lieu du filet.
 * Réservé à une section par page : c'est ce qui la distingue, et deux panneaux
 * ne distinguent plus rien.
 */
function Section({
  surtitre,
  titre,
  chapo,
  info,
  aside,
  panneau = false,
  children,
}: {
  surtitre: string
  titre?: string
  chapo?: ReactNode
  /** Mentions de la section que le téléphone ne montre que dans l'infobulle. */
  info?: ReactNode
  aside?: ReactNode
  panneau?: boolean
  children: ReactNode
}) {
  const aide = (chapo || info) && (
    <InfoTip label={`En savoir plus : ${surtitre}`} className="md:hidden">
      {chapo && <p>{chapo}</p>}
      {info}
    </InfoTip>
  )

  const coiffe = Boolean(titre || chapo)

  return (
    <section
      className={
        panneau
          ? 'rounded-2xl border border-line bg-surface-soft px-5 py-7 sm:px-8 sm:py-9'
          : 'relative border-t border-line-strong pt-8'
      }
    >
      {/* Amorce posée sur le filet : deux pixels d'accent suffisent à faire
          lire la ligne comme un début de section et non comme une clôture. */}
      {!panneau && (
        <span
          className="absolute left-0 top-[-1.5px] h-[3px] w-14 rounded-full bg-accent"
          aria-hidden
        />
      )}

      <div className="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1">
        <h3 className="text-xs font-semibold uppercase tracking-[0.08em] text-accent">
          {surtitre}
          {!titre && aide}
        </h3>
        {aside}
      </div>

      {titre && (
        <p className="mt-2 max-w-3xl text-2xl font-semibold tracking-tight text-ink">
          {titre}
          {aide}
        </p>
      )}
      {chapo && (
        <p className="mt-3 hidden max-w-3xl text-base leading-relaxed text-ink-soft md:block">
          {chapo}
        </p>
      )}

      <div className={coiffe ? 'mt-6' : 'mt-3'}>{children}</div>
    </section>
  )
}

const NEVER_SHOWN =
  'Ce qui n’est jamais affiché : une motivation reconstituée, un commentaire de presse, une synthèse rédigée par un modèle de langage. Seules comptent les positions attribuées et publiques.'

function ProcedureHelp() {
  return (
    <>
      Rien n’est supposé acquis. Non-votant, abstention, scrutin solennel,
      motion de censure : chaque mot de la procédure est expliqué au fil des
      pages, en deux niveaux de lecture, sur{' '}
      <Link to="/comprendre" className="text-accent hover:underline">
        Comprendre
      </Link>
      .
    </>
  )
}

/**
 * À propos : la méthode, la neutralité, les sources et les crédits. La
 * promesse, les usages et le dernier vote sont sur l'accueil (`LandingPage`) ;
 * cette page en est la suite pour qui veut vérifier comment le site tient ses
 * règles.
 */
export default function AboutPage() {
  return (
    <div className="space-y-12">
      <header className="max-w-3xl">
        <p className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
          À propos
        </p>
        <h2 className="mt-3 text-3xl font-semibold leading-[1.15] tracking-tight sm:text-4xl">
          Méthode, neutralité et sources
        </h2>
        <p className="mt-4 text-base leading-relaxed text-ink-soft">
          Les votes des députés sont publics et horodatés, mais dispersés dans
          des jeux de données que personne ne consulte. Ce site les rassemble :
          voici les textes, voici comment chaque groupe a voté, voici la source
          officielle.
        </p>
        <p className="mt-3 text-sm leading-relaxed text-ink-soft">
          <ProcedureHelp />
        </p>
      </header>

      {/* ---------------------------------------------------- Le « pourquoi » */}
      <Section
        info={<p>{NEVER_SHOWN}</p>}
        surtitre="Le « pourquoi » d’un vote"
        titre="Un décompte ne dit pas les raisons. Les pièces, si."
        chapo="Un vote isolé se prête à toutes les lectures. Le site ne reconstitue pas les motivations d’un groupe et ne les résume pas à sa place : il rassemble ce qui est attribué, daté et public autour de chaque scrutin, pour que le vote soit lu avec ce qui l’entoure."
      >
        <ul className="grid gap-4 sm:grid-cols-2">
          {PIECES.map((piece) => (
            <li key={piece.titre}>
              {/* Filet d'accent au flanc : les quatre pièces forment une
                  colonne de preuves, la marge colorée les rattache entre
                  elles sans ajouter de titre intermédiaire. */}
              <Card className="h-full border-l-2 border-l-accent/40 px-5 py-4 transition-shadow duration-200 hover:shadow-card-hover">
                <h4 className="text-sm font-semibold">{piece.titre}</h4>
                <p className="mt-1.5 text-sm leading-relaxed text-ink-soft">
                  {piece.texte}
                </p>
              </Card>
            </li>
          ))}
        </ul>

        <div className="mt-4">
          <Note desktopOnly>{NEVER_SHOWN}</Note>
        </div>
      </Section>

      {/* ------------------------------------------------------- Neutralité */}
      {/*
        La neutralité ne se déclare pas, elle s’expose : d’abord les trois
        contraintes qui la tiennent, ensuite la liste de ce qui est refusé —
        c’est elle qui rend la première vérifiable (README.md §1, §6).
      */}
      <Section
        panneau
        surtitre="Neutralité"
        titre="Ce site n’aide aucun groupe, et n’en vise aucun"
        chapo="Un outil qui rapproche votes et forces politiques n’a de valeur que s’il traite les deux bords à l’identique — même page, mêmes chiffres, mêmes sources. Le test appliqué partout tient en une question : une personne de droite et une personne de gauche la trouveraient-elles juste ? Trois contraintes le rendent tenable."
      >
        <ol className="grid gap-4 md:grid-cols-3">
          {PRINCIPE.map((etape, index) => (
            <li key={etape.titre}>
              <Card className="flex h-full flex-col px-5 py-4 transition-shadow duration-200 hover:shadow-card-hover">
                {/* L'étape où un jugement entre porte la teinte « info »,
                    celle des mentions de méthode ailleurs sur le site : la
                    couleur signale l'exception au lieu de la décorer. */}
                <span
                  className={`inline-flex h-6 w-6 items-center justify-center rounded-md text-[11px] font-semibold ring-1 ring-inset ${
                    etape.jugement
                      ? 'bg-info-soft text-info ring-info/20'
                      : 'bg-accent-soft text-accent ring-accent/15'
                  }`}
                >
                  {String(index + 1).padStart(2, '0')}
                </span>
                <h4 className="mt-2.5 text-sm font-semibold">{etape.titre}</h4>
                <p className="mt-2 text-sm leading-relaxed text-ink-soft">
                  {etape.texte}
                </p>
                {etape.jugement && (
                  <p className="mt-auto pt-3 text-xs leading-relaxed text-ink-faint">
                    Seul endroit du produit où un jugement intervient.{' '}
                    <Link
                      to="/themes/methode"
                      className="text-accent hover:underline"
                    >
                      La méthode
                    </Link>
                  </p>
                )}
              </Card>
            </li>
          ))}
        </ol>

        <div className="mt-6 grid gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.35fr)]">
          <div>
            <h4 className="text-sm font-semibold text-ink">
              Ce que ce site ne fait pas
              <InfoTip label="Pourquoi cette liste" className="md:hidden">
                Une promesse de neutralité sans contrepartie n’engage à rien. La
                liste ci-contre est cette contrepartie : chaque ligne est une
                fonctionnalité écartée, y compris quand elle serait facile.
              </InfoTip>
            </h4>
            <p className="mt-1.5 hidden text-sm leading-relaxed text-ink-soft md:block">
              Une promesse de neutralité sans contrepartie n’engage à rien. La
              liste ci-contre est cette contrepartie : chaque ligne est une
              fonctionnalité écartée, y compris quand elle serait facile.
            </p>
          </div>
          <ul className="space-y-2">
            {ANTI_PROMESSES.map((regle) => (
              <li
                key={regle}
                className="rounded-lg border-l-2 border-accent/40 bg-surface px-4 py-2.5 text-sm leading-relaxed text-ink-soft shadow-card"
              >
                {regle}
              </li>
            ))}
          </ul>
        </div>
      </Section>

      {/* --------------------------------------------------- Code source */}
      <Section
        surtitre="Transparence"
        titre="La méthode se consulte aussi dans le code"
        chapo="Le dépôt public permet de vérifier comment les données sont ingérées, rattachées à des thèmes et affichées. Il complète la méthode publiée ; pour chaque chiffre, la source officielle de l’Assemblée nationale reste la référence."
      >
        <Card className="px-5 py-4">
          <p className="max-w-4xl text-sm leading-relaxed text-ink-soft">
            Le code source, les migrations et l’historique des évolutions sont
            accessibles publiquement. Vous pouvez les consulter, signaler une
            erreur ou proposer une amélioration dans le dépôt.
          </p>
          <a
            href={GITHUB_REPOSITORY_URL}
            target="_blank"
            rel="noreferrer"
            className="mt-3 inline-block text-sm font-medium text-accent hover:underline"
          >
            Consulter le dépôt sur GitHub ↗
          </a>
        </Card>
      </Section>

      {/* ---------------------------------------------------------- Sources */}
      <Section surtitre="Sources">
        <Card className="px-5 py-4">
          <p className="max-w-4xl text-sm leading-relaxed text-ink-soft">
            Les données proviennent de l’open data de l’Assemblée nationale, sous
            Licence Ouverte : scrutins et positions nominales, référentiel des
            députés et des groupes avec leurs appartenances datées, dossiers
            législatifs. Rien n’est saisi à la main, rien n’est corrigé en
            silence. Le Sénat est hors périmètre.
          </p>
          <a
            href="https://data.assemblee-nationale.fr"
            target="_blank"
            rel="noreferrer"
            className="mt-2 inline-block text-sm font-medium text-accent hover:underline"
          >
            data.assemblee-nationale.fr ↗
          </a>
          <details className="mt-4 border-t border-line pt-3 text-sm text-ink-soft">
            <summary className="cursor-pointer font-medium text-ink">Crédits des logos et des portraits</summary>
            <p className="mt-2 leading-relaxed">
              Les logos des groupes et des partis sont reproduits depuis Wikimedia
              Commons, sous la licence indiquée. Ils identifient une organisation et
              n’engagent aucune d’elles envers ce site.
            </p>
            <ul className="mt-2 space-y-1 text-xs">
              {ALL_LOGOS.map((logo) => (
                <li key={logo.src}>
                  <a href={logo.credit.page} target="_blank" rel="noreferrer" className="text-accent hover:underline">
                    {logo.credit.file}
                  </a>{' '}
                  · {logo.credit.author} · {logo.credit.license}
                </li>
              ))}
            </ul>
            <p className="mt-3 leading-relaxed">
              Les portraits des candidats viennent aussi de Wikimedia Commons. Ils
              sont réduits et réencodés, et datés de leur prise de vue.
            </p>
            <ul className="mt-2 space-y-1 text-xs">
              {Object.values(PORTRAITS).map((portrait) => (
                <li key={portrait.src}>
                  <a href={portrait.credit.page} target="_blank" rel="noreferrer" className="text-accent hover:underline">
                    {portrait.credit.file}
                  </a>{' '}
                  · {portrait.credit.author} · {portrait.credit.license}
                </li>
              ))}
            </ul>
          </details>
        </Card>
      </Section>
    </div>
  )
}
