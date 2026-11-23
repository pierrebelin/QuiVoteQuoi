import { useIsFetching } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'
import { Link, useLocation } from 'react-router'
import LireUnVote from '../components/comprendre/LireUnVote'
import { RichText } from '../components/comprendre/RichText'
import { SchemaView } from '../components/comprendre/Schemas'
import { FreshnessSection } from '../components/Freshness'
import { Icon } from '../components/icons'
import { SECTIONS, type Bloc, type Question, type Section } from '../content/comprendre'

/** Section qui porte chaque question : une ancre vers une question de détail
 * doit déplier le bloc « Pour aller plus loin » qui la contient. */
const SECTION_DE = new Map(SECTIONS.flatMap((s) => s.questions.map((q) => [q.id, s.id] as const)))
const QUESTION_DE = new Map(SECTIONS.flatMap((s) => s.questions.map((q) => [q.id, q] as const)))

function BlocView({ bloc }: { bloc: Bloc }) {
  if (bloc.kind === 'p') {
    return (
      <p className="text-[15px] leading-[1.55] text-ink-soft">
        <RichText texte={bloc.texte} />
      </p>
    )
  }

  if (bloc.kind === 'ul') {
    return (
      <div>
        {bloc.intro && <p className="text-[15px] leading-[1.55] text-ink-soft">{bloc.intro}</p>}
        <ul className="mt-2 space-y-2">
          {bloc.items.map((item, index) => (
            <li
              key={index}
              className="border-l-2 border-line pl-3 text-[15px] leading-[1.55] text-ink-soft"
            >
              <RichText texte={item} />
            </li>
          ))}
        </ul>
      </div>
    )
  }

  if (bloc.kind === 'source') {
    return (
      <p className="text-[15px] leading-[1.55] text-ink-soft">
        <a href={bloc.lien} target="_blank" rel="noreferrer" className="link">
          {bloc.libelle}
        </a>
        {bloc.precision && <> — {bloc.precision}</>}
      </p>
    )
  }

  return <SchemaView bloc={bloc} />
}

function QuestionView({
  question,
  ouverte,
  onBascule,
}: {
  question: Question
  ouverte: boolean
  onBascule: (ouvert: boolean) => void
}) {
  return (
    <details
      id={question.id}
      open={ouverte}
      onToggle={(event) => onBascule(event.currentTarget.open)}
      className="scroll-mt-20 border-b border-line last:border-b-0"
    >
      <summary className="flex min-h-12 cursor-pointer list-none items-start justify-between gap-3 py-3 text-[15px] leading-[1.35] font-semibold text-ink marker:content-none">
        <span className="text-pretty">{question.question}</span>
        <Icon
          name="down"
          className={`mt-px size-[18px] text-ink-faint transition-transform ${ouverte ? 'rotate-180' : ''}`}
        />
      </summary>
      <div className="space-y-3.5 pb-4">
        {question.reponse.map((bloc, index) => (
          <BlocView key={index} bloc={bloc} />
        ))}
      </div>
    </details>
  )
}

/** Détail d'une section, replié : il suppose l'essentiel acquis. */
function PlusLoin({
  questions,
  ouvert,
  onBascule,
  children,
}: {
  questions: Question[]
  ouvert: boolean
  onBascule: (ouvert: boolean) => void
  children: React.ReactNode
}) {
  const nombre = `${questions.length} question${questions.length > 1 ? 's' : ''}`
  return (
    <details
      open={ouvert}
      onToggle={(event) => onBascule(event.currentTarget.open)}
      className="mt-1"
    >
      <summary className="flex min-h-12 cursor-pointer list-none items-center justify-between gap-3 rounded-[14px] bg-surface px-3.5 text-sm text-ink-faint marker:content-none">
        <span>
          <span className="font-semibold text-ink">Pour aller plus loin</span> · {nombre}
        </span>
        <Icon name="down" className={`size-[18px] transition-transform ${ouvert ? 'rotate-180' : ''}`} />
      </summary>
      <div className="px-3.5 pt-1">{children}</div>
    </details>
  )
}

function SectionView({
  section,
  ouvertes,
  onBascule,
}: {
  section: Section
  ouvertes: Set<string>
  onBascule: (id: string, ouvert: boolean) => void
}) {
  const essentiel = section.questions.filter((q) => q.niveau === 'debutant')
  const detail = section.questions.filter((q) => q.niveau === 'detaille')
  const vue = (question: Question) => (
    <QuestionView
      key={question.id}
      question={question}
      ouverte={ouvertes.has(question.id)}
      onBascule={(ouvert) => onBascule(question.id, ouvert)}
    />
  )
  return (
    <section id={section.id} className="mb-8 scroll-mt-20 break-inside-avoid">
      <h3 className="mb-1.5 text-[13px] font-semibold text-ink-faint">{section.titre}</h3>
      <div className="border-t border-line">
        {essentiel.map(vue)}
        {detail.length > 0 && (
          <PlusLoin
            questions={detail}
            ouvert={ouvertes.has(section.id)}
            onBascule={(ouvert) => onBascule(section.id, ouvert)}
          >
            {detail.map(vue)}
          </PlusLoin>
        )}
      </div>
    </section>
  )
}

/**
 * Guide de lecture, en questions — README.md §2, §3, §7, §9.
 *
 * En tête, « Lire un vote » explique la carte du dernier vote sur l'ensemble
 * d'un texte, servie en direct : c'est la seule donnée de la page, et elle
 * n'est jamais recopiée dans le texte. Puis les questions : l'essentiel
 * visible, le détail replié en fin de section.
 *
 * À partir de 1024 px, les sections passent sur deux colonnes : une ligne de
 * mille deux cents pixels ne se lit pas.
 */
export default function ComprendrePage() {
  const { hash } = useLocation()
  /** Questions dépliées, et sections dont le détail est déplié (par leur id). */
  const [ouvertes, setOuvertes] = useState<Set<string>>(new Set())
  /** Dernière ancre rejointe : empêche de re-défiler à chaque rendu. */
  const ancreTraitee = useRef<string | null>(null)
  /** Requêtes en vol (la carte de « Lire un vote », l'hémicycle) : tant qu'il
   * y en a, la page grandit encore au-dessus des questions. */
  const enCours = useIsFetching() > 0

  /**
   * Un renvoi venu d'une page de vote arrive par navigation cliente : la
   * question n'existe pas encore quand le navigateur traite l'ancre, et rien ne
   * défile. On la déplie, avec le bloc de détail qui la contient s'il y a lieu,
   * puis on rejoue le saut une fois le contenu rendu.
   */
  useEffect(() => {
    if (!hash) return
    const id = hash.slice(1)
    if (ancreTraitee.current === id) return

    const question = QUESTION_DE.get(id)
    if (question) {
      const aOuvrir = question.niveau === 'detaille' ? [id, SECTION_DE.get(id)!] : [id]
      setOuvertes((prev) =>
        aOuvrir.every((cle) => prev.has(cle)) ? prev : new Set([...prev, ...aOuvrir]),
      )
    }

    // La carte de « Lire un vote » arrive par requête, au-dessus des questions :
    // défiler avant qu'elle soit rendue laisserait la cible hors écran.
    if (enCours) return

    // Le dépliage modifie encore la hauteur au commit suivant : on attend que la
    // mise en page soit stabilisée. L'ancre n'est tenue pour rejointe qu'une
    // fois le saut fait, sans quoi un effet annulé ne serait jamais rejoué.
    const frame = requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        const cible = document.getElementById(id)
        if (!cible) return
        cible.scrollIntoView()
        ancreTraitee.current = id
      }),
    )
    return () => cancelAnimationFrame(frame)
  }, [hash, enCours])

  function basculer(id: string, ouvert: boolean) {
    setOuvertes((prev) => {
      if (prev.has(id) === ouvert) return prev
      const suivant = new Set(prev)
      if (ouvert) suivant.add(id)
      else suivant.delete(id)
      return suivant
    })
  }

  return (
    <div className="space-y-10 lg:space-y-14">
      <div>
        <h2 className="heading-2 md:heading-1">Comprendre</h2>
        <p className="mt-2 text-base leading-normal text-ink-soft md:mt-3 md:max-w-2xl md:text-lg md:leading-relaxed">
          Ce que ce site affiche, ce que les mots recouvrent, et ce que les
          données ne permettent pas de dire.
        </p>
      </div>

      <LireUnVote />

      <section aria-labelledby="toutes-les-questions" className="lg:border-t lg:border-line lg:pt-7">
        <h3 id="toutes-les-questions" className="heading-4 md:heading-3">
          Toutes les questions
        </h3>
        <p className="mt-1 mb-6 text-[13px] leading-[19px] text-ink-faint md:text-sm md:leading-5">
          L’essentiel d’abord ; le détail, replié en fin de section, suppose l’essentiel acquis.
        </p>
        <div className="lg:columns-2 lg:gap-14">
          {SECTIONS.map((section) => (
            <SectionView
              key={section.id}
              section={section}
              ouvertes={ouvertes}
              onBascule={basculer}
            />
          ))}
        </div>
      </section>

      <FreshnessSection />

      <p className="text-sm text-ink-soft">
        La méthode de rattachement aux sujets est publiée à part :{' '}
        <Link to="/themes/methode" className="link">
          méthode de thématisation
        </Link>
        .
      </p>
    </div>
  )
}
