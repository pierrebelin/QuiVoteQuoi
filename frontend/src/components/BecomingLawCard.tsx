import type { ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { LawStateRow } from './LawStateList'
import type { DossierCardDto, DossierPageResponse } from './DossierCard'
import { formatShortDate } from '../lib/dossierStatus'
import { fetchJson } from '../lib/fetchJson'
import { LAW_STATE_FILTER_LABELS, lawStateDetail } from '../lib/lawState'
import { votesWord } from '../lib/week'
import type { LawStateKind } from '../types/dossiers'

/** Plafond de l'API par page : au-delà, un lien mène au reste. */
const ADOPTED_LIMIT = 100
/** Les derniers textes devenus loi, par date de promulgation (§4, Accueil). */
const LAWS_LIMIT = 2
/**
 * À partir de 1280 px, la carte tient à côté de la semaine : les quatre textes
 * adoptés les plus récents, titres coupés à deux lignes, puis un lien vers les
 * autres. En dessous, tous restent affichés.
 */
const ADOPTED_WIDE = 4
const WIDE_TITLE = 'xl:line-clamp-2'

/**
 * Tous les dossiers, sans le filtre des textes votés de la liste : un texte
 * adopté sans scrutin public compte aussi (README §2). Le plus récent d'abord,
 * par la date de l'état (la décision qui donne le même texte, ou la
 * promulgation).
 */
const statePage = (state: LawStateKind, perPage: number) =>
  fetchJson<DossierPageResponse>(`/api/dossiers?state=${state}&sort=state_date&per_page=${perPage}`)

/** Liste par texte, tous les dossiers, filtrée sur un état. */
const listPath = (stateParam: string) => `/scrutins?vue=texte&textes=tous&etat=${stateParam}`

const capitalize = (text: string) => text.charAt(0).toUpperCase() + text.slice(1)

/** Intitulé d'un groupe de textes : le libellé de la bande, sans majuscules ni espacement. */
function GroupLabel({ children, className = '' }: { children: ReactNode; className?: string }) {
  return (
    <h4 className={`mb-2 text-[13px] leading-[18px] font-semibold text-ink-faint ${className}`}>{children}</h4>
  )
}

function AdoptedRow({
  dossier,
  today,
  hiddenWide,
}: {
  dossier: DossierCardDto
  today: string
  /** Au-delà des quatre premiers : masqué à partir de 1280 px. */
  hiddenWide: boolean
}) {
  const date = dossier.law_state ? formatShortDate(dossier.law_state.date, today) : null
  const votes = dossier.scrutin_count > 0 ? `${votesWord(dossier.scrutin_count)} à l’Assemblée` : null
  return (
    <LawStateRow
      kind="definitively_adopted"
      dossierUid={dossier.uid}
      title={dossier.title}
      className={hiddenWide ? 'xl:hidden' : ''}
      titleClassName={WIDE_TITLE}
    >
      {[date, votes].filter(Boolean).join(' · ')}
    </LawStateRow>
  )
}

function LawRow({ dossier, today }: { dossier: DossierCardDto; today: string }) {
  const detail = dossier.law_state ? lawStateDetail(dossier.law_state, today) : null
  return (
    <LawStateRow kind="became_law" dossierUid={dossier.uid} title={dossier.title} titleClassName={WIDE_TITLE}>
      {detail && capitalize(detail)}
    </LawStateRow>
  )
}

/**
 * « Ce qui devient loi » (SPEC-navette §4, Q1) : tous les textes adoptés
 * définitivement, puis les deux derniers devenus loi, puis un lien vers la
 * liste filtrée sur Devenus loi. Couleurs et icônes de RM-07. À partir de
 * 1280 px, en deux colonnes à droite de la semaine (accueil, variante C du
 * 22/11/2026), voir ADOPTED_WIDE. Rien ne
 * s'affiche tant que les deux listes ne sont pas arrivées, ni si l'une échoue :
 * une carte à moitié remplie dirait à tort qu'il n'y a rien.
 */
export default function BecomingLawCard() {
  const adopted = useQuery({
    queryKey: ['dossiers', 'accueil', 'definitively_adopted'],
    queryFn: () => statePage('definitively_adopted', ADOPTED_LIMIT),
    retry: false,
  })
  const laws = useQuery({
    queryKey: ['dossiers', 'accueil', 'became_law'],
    queryFn: () => statePage('became_law', LAWS_LIMIT),
    retry: false,
  })
  if (!adopted.data || !laws.data) return null

  const adoptedItems = adopted.data.dossiers
  const adoptedTotal = adopted.data.state_counts.definitively_adopted
  const lawItems = laws.data.dossiers
  if (adoptedItems.length === 0 && lawItems.length === 0) return null
  const today = new Date().toISOString().slice(0, 10)

  const beyondWide = adoptedTotal - ADOPTED_WIDE

  return (
    <section aria-labelledby="devient-loi" className="xl:border-l xl:border-line xl:pl-10">
      <h3 id="devient-loi" className="heading-4">
        Ce qui devient loi
      </h3>
      <p className="mt-1 text-[13px] leading-[19px] text-ink-faint xl:hidden">
        Textes votés dans les mêmes termes par l’Assemblée et le Sénat.
      </p>
      <p className="mt-1 hidden text-[13px] leading-[19px] text-ink-faint xl:block">
        Votés dans les mêmes termes par l’Assemblée et le Sénat
      </p>
      <div className="mt-4 xl:mt-5 xl:grid xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)] xl:items-start xl:gap-x-8">
        <div>
          <GroupLabel>
            {LAW_STATE_FILTER_LABELS.definitively_adopted} · promulgation attendue
          </GroupLabel>
          {adoptedItems.length > 0 ? (
            adoptedItems.map((dossier, index) => (
              <AdoptedRow key={dossier.uid} dossier={dossier} today={today} hiddenWide={index >= ADOPTED_WIDE} />
            ))
          ) : (
            <p className="border-t border-line py-[11px] text-[13px] leading-[19px] text-ink-soft">
              Aucun texte n’attend sa promulgation.
            </p>
          )}
          {adoptedTotal > adoptedItems.length && (
            <Link to={listPath('adopte')} className="mt-1 inline-block text-sm font-medium link xl:hidden">
              Les {adoptedTotal.toLocaleString('fr-FR')} textes adoptés définitivement
            </Link>
          )}
          {beyondWide > 0 && (
            <p className="hidden border-t border-line pt-2.5 pl-10 xl:block">
              <Link to={listPath('adopte')} className="text-sm font-medium link">
                et {beyondWide.toLocaleString('fr-FR')} autre{beyondWide > 1 ? 's' : ''} texte
                {beyondWide > 1 ? 's' : ''}
              </Link>
            </p>
          )}
        </div>
        {lawItems.length > 0 && (
          <div>
            <GroupLabel className="mt-8 xl:mt-0">{LAW_STATE_FILTER_LABELS.became_law} récemment</GroupLabel>
            {lawItems.map((dossier) => (
              <LawRow key={dossier.uid} dossier={dossier} today={today} />
            ))}
            <p className="mt-3.5 xl:mt-0 xl:border-t xl:border-line xl:pt-3">
              <Link to={listPath('loi')} className="inline-block text-sm font-semibold link">
                Tous les textes devenus loi →
              </Link>
            </p>
          </div>
        )}
      </div>
    </section>
  )
}
