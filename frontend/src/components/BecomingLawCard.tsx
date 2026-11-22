import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { LawStateGroupHeading, LawStateRow } from './LawStateList'
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

function AdoptedRow({ dossier, today }: { dossier: DossierCardDto; today: string }) {
  const date = dossier.law_state ? formatShortDate(dossier.law_state.date, today) : null
  const votes = dossier.scrutin_count > 0 ? `${votesWord(dossier.scrutin_count)} à l’Assemblée` : null
  return (
    <LawStateRow kind="definitively_adopted" dossierUid={dossier.uid} title={dossier.title}>
      {[date, votes].filter(Boolean).join(' · ')}
    </LawStateRow>
  )
}

function LawRow({ dossier, today }: { dossier: DossierCardDto; today: string }) {
  const detail = dossier.law_state ? lawStateDetail(dossier.law_state, today) : null
  return (
    <LawStateRow kind="became_law" dossierUid={dossier.uid} title={dossier.title}>
      {detail && capitalize(detail)}
    </LawStateRow>
  )
}

/**
 * « Ce qui devient loi » (SPEC-navette §4, Q1) : tous les textes adoptés
 * définitivement, puis les deux derniers devenus loi, puis un lien vers la
 * liste filtrée sur Devenus loi. Couleurs et icônes de RM-07. Rien ne
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

  return (
    <section aria-labelledby="devient-loi" className="space-y-2.5">
      <h3 id="devient-loi" className="heading-4">
        Ce qui devient loi
      </h3>
      <div className="rounded border border-line bg-surface p-4 md:px-6 md:py-5 xl:p-5">
        <p className="mb-3.5 text-[13px] leading-[19px] text-ink-soft">
          Textes votés dans les mêmes termes par l’Assemblée et le Sénat.
        </p>
        <LawStateGroupHeading kind="definitively_adopted" first>
          {LAW_STATE_FILTER_LABELS.definitively_adopted} · promulgation attendue
        </LawStateGroupHeading>
        {adoptedItems.length > 0 ? (
          adoptedItems.map((dossier) => <AdoptedRow key={dossier.uid} dossier={dossier} today={today} />)
        ) : (
          <p className="border-t border-line py-[11px] text-[13px] leading-[19px] text-ink-soft">
            Aucun texte n’attend sa promulgation.
          </p>
        )}
        {adoptedTotal > adoptedItems.length && (
          <Link to={listPath('adopte')} className="mt-1 inline-block text-sm font-medium link">
            Les {adoptedTotal.toLocaleString('fr-FR')} textes adoptés définitivement
          </Link>
        )}
        {lawItems.length > 0 && (
          <>
            <LawStateGroupHeading kind="became_law">
              {LAW_STATE_FILTER_LABELS.became_law} récemment
            </LawStateGroupHeading>
            {lawItems.map((dossier) => (
              <LawRow key={dossier.uid} dossier={dossier} today={today} />
            ))}
            <Link to={listPath('loi')} className="mt-3.5 inline-block text-sm font-semibold link">
              Tous les textes devenus loi →
            </Link>
          </>
        )}
      </div>
    </section>
  )
}
