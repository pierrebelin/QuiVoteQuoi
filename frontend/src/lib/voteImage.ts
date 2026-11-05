import { LOGO_TRACES, LOGO_VUE } from '../components/logoTraces'
import { PLAN_SIEGES, PLAN_VUE, RAYON_SIEGE } from '../components/planHemicycle'
import { siegesDesGroupes } from '../components/sieges'
import type {
  CitedAmendmentDto,
  GroupBreakdownDto,
  ScrutinDetailDto,
  TallyDto,
  VoteDto,
} from '../types/scrutins'
import { formatDate, tallySize } from '../types/scrutins'

/**
 * Image de partage d'un scrutin, dessinée dans un canvas.
 *
 * Dessinée plutôt que capturée depuis la page : une capture du DOM dépend de la
 * largeur de l'écran du lecteur, du thème qu'il a choisi et de ce qu'il a
 * déplié. L'image publiée sur un réseau social doit être la même pour tous, aux
 * dimensions que ce réseau attend.
 *
 * Elle ne dit que ce que la page dit (README.md §6) : le résultat en nombres,
 * jamais en adverbes, le groupe sous son sigle officiel, et la source avec
 * l'adresse de la page — une image circule sans son contexte, c'est donc elle
 * qui doit le porter.
 */

export type ReseauImage =
  | 'instagram'
  | 'story'
  | 'reel'
  | 'tiktok'
  | 'linkedin'
  | 'facebook'
  | 'threads'
  | 'reddit'
  | 'x'
  | 'bluesky'
  | 'mastodon'
export type VisuelImage = 'hemicycle' | 'groupes'
export type ThemeImage = 'clair' | 'sombre'

export type OptionsImage = {
  reseau: ReseauImage
  visuel: VisuelImage
  theme: ThemeImage
  /** Adresse de la page du scrutin, imprimée en pied d'image. */
  lien: string
}

type Gabarit = 'portrait' | 'carre' | 'paysage' | 'vertical' | 'story'

/**
 * Géométrie d'une image. `u` est l'échelle des tailles de texte ; les maxima
 * de lignes tronquent un intitulé trop long plutôt que de pousser le visuel
 * hors du cadre — l'intitulé entier reste sur la page dont l'adresse est
 * imprimée.
 *
 * Le vertical réserve les zones que Reels, Stories et TikTok couvrent de leur
 * interface : le nom du compte en haut, la légende et les boutons en bas, la
 * colonne d'icônes à droite. Le fond les remplit, le contenu les évite.
 *
 * La story n'a pas cette colonne d'icônes : elle garde les seules zones du haut
 * et du bas, et son texte prend toute la largeur. Son dessin est à part, voir
 * `dessinerStoryVote`.
 */
const GABARITS: Record<
  Gabarit,
  {
    largeur: number
    hauteur: number
    u: number
    marges: { haut: number; bas: number; gauche: number; droite: number }
    lignesTitre: number
    lignesSujet: number
  }
> = {
  portrait: {
    largeur: 1080,
    hauteur: 1350,
    u: 1,
    marges: { haut: 72, bas: 72, gauche: 64, droite: 64 },
    lignesTitre: 3,
    lignesSujet: 3,
  },
  carre: {
    largeur: 1080,
    hauteur: 1080,
    u: 1,
    marges: { haut: 64, bas: 64, gauche: 64, droite: 64 },
    lignesTitre: 2,
    lignesSujet: 2,
  },
  paysage: {
    largeur: 1200,
    hauteur: 675,
    u: 0.8,
    marges: { haut: 52, bas: 52, gauche: 52, droite: 52 },
    lignesTitre: 3,
    lignesSujet: 2,
  },
  vertical: {
    largeur: 1080,
    hauteur: 1920,
    u: 1.05,
    marges: { haut: 250, bas: 440, gauche: 72, droite: 140 },
    lignesTitre: 4,
    lignesSujet: 3,
  },
  story: {
    largeur: 1080,
    hauteur: 1920,
    u: 1,
    marges: { haut: 250, bas: 340, gauche: 72, droite: 72 },
    lignesTitre: 5,
    lignesSujet: 5,
  },
}

/**
 * Un réseau, un gabarit : celui que le réseau affiche sans rogner dans son
 * fil. L'ordre est celui du sélecteur.
 */
export const RESEAUX: Record<
  ReseauImage,
  {
    libelle: string
    gabarit: Gabarit
    usage: string
    /** Longueur maximale d'un texte, pour les réseaux où elle est courte. */
    limiteTexte?: number
  }
> = {
  instagram: { libelle: 'Instagram', gabarit: 'portrait', usage: 'Publication du fil, 4:5' },
  story: { libelle: 'Story', gabarit: 'story', usage: 'Instagram, 9:16, le vote puis l’exposé s’il existe' },
  reel: { libelle: 'Reel', gabarit: 'vertical', usage: 'Instagram et Facebook, 9:16, hors zones des boutons' },
  tiktok: { libelle: 'TikTok', gabarit: 'vertical', usage: 'Mode photo, 9:16, hors zones des boutons' },
  linkedin: { libelle: 'LinkedIn', gabarit: 'portrait', usage: 'Image du fil, 4:5' },
  facebook: { libelle: 'Facebook', gabarit: 'portrait', usage: 'Publication du fil, 4:5' },
  threads: { libelle: 'Threads', gabarit: 'portrait', usage: 'Publication du fil, 4:5', limiteTexte: 500 },
  reddit: { libelle: 'Reddit', gabarit: 'carre', usage: 'Publication image, 1:1' },
  x: { libelle: 'X', gabarit: 'paysage', usage: 'Image du fil, 16:9', limiteTexte: 280 },
  bluesky: { libelle: 'Bluesky', gabarit: 'paysage', usage: 'Image du fil, 16:9', limiteTexte: 300 },
  mastodon: { libelle: 'Mastodon', gabarit: 'paysage', usage: 'Image du fil, 16:9', limiteTexte: 500 },
}

/** Dimensions en pixels de l'image d'un réseau. */
export function dimensions(reseau: ReseauImage) {
  const { largeur, hauteur } = GABARITS[RESEAUX[reseau].gabarit]
  return { largeur, hauteur }
}

/**
 * Reprise des tokens de `index.css`. Le canvas ne lit pas les variables CSS
 * d'un thème qui n'est pas celui de la page : on peut exporter en sombre
 * depuis une page claire.
 */
const PALETTES = {
  clair: {
    fond: '#ffffff',
    fondDoux: '#f4f4f5',
    trait: '#e8e8ea',
    traitFort: '#d1d1d6',
    encre: '#18181b',
    encreDouce: '#52525b',
    encrePale: '#8e8e97',
    accent: '#2563eb',
    pour: '#04785d',
    pourDoux: '#edfaf5',
    pourBarre: '#14a37f',
    contre: '#b3261e',
    contreDoux: '#fdf1f0',
    contreBarre: '#d9534b',
    abstention: '#9a5b06',
    abstentionBarre: '#d99b31',
    siegeVide: '#ededef',
  },
  sombre: {
    fond: '#151519',
    fondDoux: '#1e1e24',
    trait: '#27272e',
    traitFort: '#3b3b45',
    encre: '#f4f4f5',
    encreDouce: '#a1a1aa',
    encrePale: '#71717a',
    accent: '#60a5fa',
    pour: '#34d399',
    pourDoux: 'rgba(4, 120, 87, 0.2)',
    pourBarre: '#12977a',
    contre: '#fca5a5',
    contreDoux: 'rgba(153, 27, 27, 0.22)',
    contreBarre: '#cf4f47',
    abstention: '#fbbf24',
    abstentionBarre: '#c98f2d',
    siegeVide: '#23232a',
  },
} as const

type Palette = (typeof PALETTES)[ThemeImage]

const POLICE = '"Inter Variable", ui-sans-serif, system-ui, sans-serif'

function police(ctx: CanvasRenderingContext2D, poids: number, taille: number) {
  ctx.font = `${poids} ${taille}px ${POLICE}`
}

/**
 * Inter est chargée à la demande par le navigateur : un canvas dessiné avant
 * qu'elle arrive retombe sur la police système, et l'image téléchargée aussi.
 */
export async function chargerPolices() {
  try {
    await Promise.all(
      [400, 600, 700].map((poids) =>
        document.fonts.load(`${poids} 32px "Inter Variable"`),
      ),
    )
  } catch {
    // Police indisponible : l'image se dessine avec la police système.
  }
}

/* ------------------------------------------------------------------ texte */

function couperLignes(
  ctx: CanvasRenderingContext2D,
  texte: string,
  largeur: number,
  maxLignes: number,
): string[] {
  const lignes: string[] = []
  let courante = ''
  for (const mot of texte.split(/\s+/).filter(Boolean)) {
    const essai = courante ? `${courante} ${mot}` : mot
    if (courante && ctx.measureText(essai).width > largeur) {
      lignes.push(courante)
      courante = mot
    } else {
      courante = essai
    }
  }
  if (courante) lignes.push(courante)
  if (lignes.length <= maxLignes) return lignes

  const gardees = lignes.slice(0, maxLignes)
  let derniere = gardees[maxLignes - 1]
  while (derniere.includes(' ') && ctx.measureText(`${derniere}…`).width > largeur) {
    derniere = derniere.replace(/\s+\S+$/, '')
  }
  gardees[maxLignes - 1] = `${derniere}…`
  return gardees
}

/** Écrit un paragraphe et renvoie la hauteur occupée. */
function paragraphe(
  ctx: CanvasRenderingContext2D,
  texte: string,
  x: number,
  y: number,
  largeur: number,
  maxLignes: number,
  interligne: number,
) {
  const lignes = couperLignes(ctx, texte, largeur, maxLignes)
  ctx.textBaseline = 'top'
  lignes.forEach((ligne, i) => ctx.fillText(ligne, x, y + i * interligne))
  return lignes.length * interligne
}

type Morceau = { texte: string; couleur: string; poids?: number }

/** Une ligne en plusieurs couleurs, alignée à gauche ou à droite de `x`. */
function morceaux(
  ctx: CanvasRenderingContext2D,
  liste: Morceau[],
  x: number,
  y: number,
  taille: number,
  alignement: 'gauche' | 'droite' = 'gauche',
) {
  const largeurs = liste.map((morceau) => largeurMorceaux(ctx, [morceau], taille))
  let curseur =
    alignement === 'droite' ? x - largeurs.reduce((a, b) => a + b, 0) : x
  ctx.textBaseline = 'middle'
  liste.forEach(({ texte, couleur, poids = 400 }, i) => {
    police(ctx, poids, taille)
    ctx.fillStyle = couleur
    ctx.fillText(texte, curseur, y)
    curseur += largeurs[i]
  })
}

/* ---------------------------------------------------------------- dessins */

function logo(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  hauteur: number,
  surFondSombre: boolean,
) {
  if (!surFondSombre) {
    const echelle = hauteur / LOGO_VUE.hauteur
    ctx.save()
    ctx.translate(x, y)
    ctx.scale(echelle, echelle)
    LOGO_TRACES.forEach(([d, couleur]) => {
      ctx.fillStyle = couleur
      ctx.fill(new Path2D(d))
    })
    ctx.restore()
    return LOGO_VUE.largeur * echelle
  }

  // Sur fond sombre, les rayons bleu nuit disparaissent : le logo prend la
  // pastille crème du favicon, rayons centraux en blanc comme lui.
  const cote = hauteur * 1.15
  const haut = y + (hauteur - cote) / 2
  ctx.fillStyle = '#ded3bd'
  ctx.beginPath()
  ctx.roundRect(x, haut, cote, cote, cote * 0.19)
  ctx.fill()

  const largeur = cote * 0.82
  const echelle = largeur / LOGO_VUE.largeur
  ctx.save()
  ctx.translate(
    x + (cote - largeur) / 2,
    haut + (cote - LOGO_VUE.hauteur * echelle) / 2,
  )
  ctx.scale(echelle, echelle)
  LOGO_TRACES.forEach(([d, couleur]) => {
    ctx.fillStyle = couleur === '#ded3bd' ? '#ffffff' : couleur
    ctx.fill(new Path2D(d))
  })
  ctx.restore()
  return cote
}

/**
 * Barre de répartition, même règle que `VoteBar` : les non-votants occupent
 * leur part réelle, et `part` réduit la longueur pour comparer des groupes.
 */
function barre(
  ctx: CanvasRenderingContext2D,
  p: Palette,
  tally: TallyDto,
  x: number,
  y: number,
  largeur: number,
  hauteur: number,
  part = 1,
) {
  const segments: [number, string][] = [
    [tally.votes_for, p.pourBarre],
    [tally.votes_against, p.contreBarre],
    [tally.abstentions, p.abstentionBarre],
    [tally.not_voting, p.traitFort],
  ]
  const total = segments.reduce((somme, [valeur]) => somme + valeur, 0)

  ctx.fillStyle = p.fondDoux
  ctx.beginPath()
  ctx.roundRect(x, y, largeur, hauteur, hauteur / 2)
  ctx.fill()
  if (total === 0) return

  ctx.save()
  ctx.beginPath()
  ctx.roundRect(x, y, largeur * part, hauteur, hauteur / 2)
  ctx.clip()
  let curseur = x
  segments.forEach(([valeur, couleur]) => {
    const l = (valeur / total) * largeur * part
    ctx.fillStyle = couleur
    ctx.fillRect(curseur, y, l, hauteur)
    curseur += l
  })
  ctx.restore()
}

const COULEUR_SIEGE: Record<VoteDto['position'], keyof Palette> = {
  for: 'pourBarre',
  against: 'contreBarre',
  abstention: 'abstentionBarre',
  not_voting: 'traitFort',
}

type Cadre = { x: number; y: number; largeur: number; hauteur: number }

/** L'hémicycle, chaque député à son siège réel — voir `Hemicycle.tsx`. */
function hemicycle(
  ctx: CanvasRenderingContext2D,
  p: Palette,
  theme: ThemeImage,
  data: ScrutinDetailDto,
  cadre: Cadre,
  u: number,
) {
  const votes = siegesDesGroupes(data.groups)
  const parSiege = new Map(
    votes.filter((v) => v.seat !== null).map((v) => [v.seat as number, v]),
  )
  const horsPlan = votes.filter(
    (v) => v.seat === null || !(v.seat in PLAN_SIEGES),
  ).length

  const legende =
    `Chaque député à son siège · en gris ${theme === 'sombre' ? 'foncé' : 'clair'}, aucune position enregistrée` +
    (horsPlan > 0
      ? ` · ${horsPlan} position${horsPlan > 1 ? 's' : ''} sans siège localisable`
      : '')
  const tailleLegende = 17 * u
  police(ctx, 400, tailleLegende)
  const lignesLegende = couperLignes(ctx, legende, cadre.largeur, 2)
  const hauteurLegende = lignesLegende.length * tailleLegende * 1.4 + 12 * u

  const hauteurDispo = cadre.hauteur - hauteurLegende
  const echelle = Math.min(
    cadre.largeur / PLAN_VUE.largeur,
    hauteurDispo / PLAN_VUE.hauteur,
  )
  const largeur = PLAN_VUE.largeur * echelle
  const hauteur = PLAN_VUE.hauteur * echelle
  const x0 = cadre.x + (cadre.largeur - largeur) / 2
  const y0 = cadre.y + (hauteurDispo - hauteur) / 2

  Object.entries(PLAN_SIEGES).forEach(([numero, [x, y]]) => {
    const vote = parSiege.get(Number(numero))
    ctx.fillStyle = vote ? p[COULEUR_SIEGE[vote.position]] : p.siegeVide
    ctx.beginPath()
    ctx.arc(x0 + x * echelle, y0 + y * echelle, RAYON_SIEGE * echelle, 0, Math.PI * 2)
    ctx.fill()
  })

  police(ctx, 400, tailleLegende)
  ctx.fillStyle = p.encrePale
  ctx.textAlign = 'center'
  ctx.textBaseline = 'top'
  lignesLegende.forEach((ligne, i) =>
    ctx.fillText(
      ligne,
      cadre.x + cadre.largeur / 2,
      y0 + hauteur + 12 * u + i * tailleLegende * 1.4,
    ),
  )
  ctx.textAlign = 'left'
}

const NOTE_RECONSTITUE =
  '* Répartition reconstituée à partir des positions nominales, non publiée par l’Assemblée.'

function nomGroupe(groupe: GroupBreakdownDto) {
  // Sigle officiel, jamais un parti (README.md §3.1).
  const nom = groupe.abbrev ?? groupe.label ?? groupe.group_uid ?? 'Sans groupe'
  return groupe.origin === 'reconstructed' ? `${nom}*` : nom
}

/**
 * Décompte d'un groupe, non-votants compris quand il y en a : la barre leur
 * donne leur part réelle, la ligne doit donc dire combien ils sont.
 *
 * `voluntary_not_voting` n'entre pas dans le compte : au niveau des groupes, il
 * reproduit les abstentions et les compterait deux fois (n° 9759 : GDR, 17
 * membres, 15 contre, 2 abstentions, aucun non-votant sur la page de l'Assemblée).
 */
function chiffresGroupe(tally: TallyDto, p: Palette): Morceau[] {
  const nonVotants = tally.not_voting
  return [
    { texte: String(tally.votes_for), couleur: p.pour, poids: 600 },
    { texte: ' pour · ', couleur: p.encrePale },
    { texte: String(tally.votes_against), couleur: p.contre, poids: 600 },
    { texte: ' contre · ', couleur: p.encrePale },
    { texte: String(tally.abstentions), couleur: p.abstention, poids: 600 },
    { texte: ' abst.', couleur: p.encrePale },
    ...(nonVotants > 0
      ? [
          { texte: ' · ', couleur: p.encrePale },
          { texte: String(nonVotants), couleur: p.encreDouce, poids: 600 },
          { texte: ` non-votant${nonVotants > 1 ? 's' : ''}`, couleur: p.encrePale },
        ]
      : []),
  ]
}

function largeurMorceaux(
  ctx: CanvasRenderingContext2D,
  liste: Morceau[],
  taille: number,
) {
  return liste.reduce((somme, { texte, poids = 400 }) => {
    police(ctx, poids, taille)
    return somme + ctx.measureText(texte).width
  }, 0)
}

/** Une ligne par groupe, les barres à l'échelle du groupe le plus nombreux. */
function groupes(
  ctx: CanvasRenderingContext2D,
  p: Palette,
  data: ScrutinDetailDto,
  cadre: Cadre,
  u: number,
) {
  const liste = data.groups
  if (liste.length === 0) return
  const reconstitue = liste.some((g) => g.origin === 'reconstructed')
  const hauteurNote = reconstitue ? 32 * u : 0

  const pas = Math.min(56 * u, (cadre.hauteur - hauteurNote) / liste.length)
  const plusGrand = Math.max(...liste.map((g) => tallySize(g.tally)), 1)

  // Le texte rétrécit tant que la barre n'a pas au moins le quart de la
  // largeur : en paysage, la colonne est étroite et les non-votants allongent
  // la ligne.
  let taille = Math.min(20 * u, pas * 0.5)
  let largeurNom = 0
  let largeurBarre = 0
  for (;;) {
    police(ctx, 600, taille)
    largeurNom = Math.max(...liste.map((g) => ctx.measureText(nomGroupe(g)).width))
    const largeurChiffres = Math.max(
      ...liste.map((g) => largeurMorceaux(ctx, chiffresGroupe(g.tally, p), taille)),
    )
    largeurBarre =
      cadre.largeur - taille * 0.64 - 10 * u - largeurNom - 18 * u - largeurChiffres - 18 * u
    if (largeurBarre >= cadre.largeur * 0.25 || taille <= 12 * u) break
    taille *= 0.92
  }
  largeurBarre = Math.max(largeurBarre, 40 * u)
  const pastille = taille * 0.32
  const debutNom = cadre.x + pastille * 2 + 10 * u
  const debutBarre = debutNom + largeurNom + 18 * u
  const y0 = cadre.y + (cadre.hauteur - hauteurNote - pas * liste.length) / 2

  liste.forEach((groupe, i) => {
    const milieu = y0 + pas * i + pas / 2

    // La couleur du groupe reste cantonnée à sa pastille (README.md §6).
    ctx.fillStyle = groupe.color ?? p.traitFort
    ctx.beginPath()
    ctx.arc(cadre.x + pastille, milieu, pastille, 0, Math.PI * 2)
    ctx.fill()

    morceaux(ctx, [{ texte: nomGroupe(groupe), couleur: p.encre, poids: 600 }], debutNom, milieu, taille)

    const hauteurBarre = Math.max(6 * u, taille * 0.45)
    barre(
      ctx,
      p,
      groupe.tally,
      debutBarre,
      milieu - hauteurBarre / 2,
      largeurBarre,
      hauteurBarre,
      Math.min(1, tallySize(groupe.tally) / plusGrand),
    )

    morceaux(ctx, chiffresGroupe(groupe.tally, p), cadre.x + cadre.largeur, milieu, taille, 'droite')
  })

  if (reconstitue) {
    morceaux(
      ctx,
      [
        {
          texte: NOTE_RECONSTITUE,
          couleur: p.encrePale,
        },
      ],
      cadre.x,
      cadre.y + cadre.hauteur - hauteurNote / 2,
      15 * u,
    )
  }
}

/* ------------------------------------------------------------------ image */

function majuscule(texte: string) {
  return texte.charAt(0).toUpperCase() + texte.slice(1)
}

/**
 * Ce qui a été mis aux voix, jamais le dossier : « Adopté » sous « Projet de loi
 * de finances » laisserait croire que le texte entier a été adopté, quand le
 * vote portait sur un amendement.
 *
 * `tete` est court quand l'intitulé commence par un amendement ou un article ;
 * sinon c'est l'intitulé entier. Le dossier n'est ajouté que si l'intitulé ne
 * nomme aucun texte.
 */
function objetDuVote(data: ScrutinDetailDto) {
  const intitule = data.subject.trim().replace(/\.$/, '')
  const decoupe = intitule.match(
    /^(?:l['’]|le\s+)((?:sous-)?amendement n°\s*\S+|article\s+\S+)\s+(.+)$/i,
  )
  const nommeUnTexte = /\b(?:projet|proposition) de loi\b/i.test(intitule)
  const dossier = data.dossier_label && !nommeUnTexte ? `Dossier : ${data.dossier_label}` : ''
  return {
    tete: majuscule(decoupe ? decoupe[1] : intitule),
    court: decoupe !== null,
    suite: [decoupe?.[2] ?? '', dossier].filter(Boolean).join(' · '),
  }
}

/**
 * Dessine l'image d'un scrutin. Synchrone : appeler `chargerPolices` avant,
 * sinon le premier rendu se fait en police système.
 */
export function dessinerVote(
  canvas: HTMLCanvasElement,
  data: ScrutinDetailDto,
  options: OptionsImage,
) {
  const gabarit = RESEAUX[options.reseau].gabarit
  if (gabarit === 'story') {
    dessinerStoryVote(canvas, data, options.lien)
    return
  }
  const {
    largeur: W,
    hauteur: H,
    u,
    marges,
    lignesTitre,
    lignesSujet,
  } = GABARITS[gabarit]
  // Bords de la zone de contenu.
  const G = marges.gauche
  const D = W - marges.droite
  const haut = marges.haut
  const bas = H - marges.bas
  const p = PALETTES[options.theme]
  const paysage = gabarit === 'paysage'

  canvas.width = W
  canvas.height = H
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  ctx.fillStyle = p.fond
  ctx.fillRect(0, 0, W, H)

  // En paysage, le texte à gauche et le visuel à droite ; sinon empilés.
  const coupure = G + Math.round((D - G) * 0.47)
  const colonne = paysage ? coupure - G : D - G
  let y = haut

  /* En-tête : identité du site, numéro et date du scrutin. */
  const hauteurLogo = 38 * u
  const largeurLogo = logo(ctx, G, y, hauteurLogo, options.theme === 'sombre')
  morceaux(
    ctx,
    [
      { texte: 'QuiVote', couleur: p.encre, poids: 600 },
      { texte: 'Quoi', couleur: p.accent, poids: 600 },
    ],
    G + largeurLogo + 12 * u,
    y + hauteurLogo * 0.6,
    26 * u,
  )
  morceaux(
    ctx,
    [{ texte: `Scrutin n° ${data.number} · ${formatDate(data.date)}`, couleur: p.encrePale }],
    D,
    y + hauteurLogo * 0.6,
    21 * u,
    'droite',
  )
  y += hauteurLogo + 34 * u

  /* Résultat, en pastille : le fait, pas un jugement. */
  const adopte = data.outcome_code === 'adopté'
  const taillePastille = 21 * u
  police(ctx, 600, taillePastille)
  const resultat = majuscule(data.outcome_code)
  const largeurPastille = ctx.measureText(resultat).width + 24 * u
  const hauteurPastille = taillePastille * 1.7
  ctx.fillStyle = adopte ? p.pourDoux : p.contreDoux
  ctx.beginPath()
  ctx.roundRect(G, y, largeurPastille, hauteurPastille, 8 * u)
  ctx.fill()
  morceaux(
    ctx,
    [{ texte: resultat, couleur: adopte ? p.pour : p.contre, poids: 600 }],
    G + 12 * u,
    y + hauteurPastille / 2,
    taillePastille,
  )
  police(ctx, 400, 19 * u)
  const procedure = couperLignes(ctx, data.ballot_type_label, colonne - largeurPastille - 14 * u, 1)[0] ?? ''
  morceaux(
    ctx,
    [{ texte: procedure, couleur: p.encrePale }],
    G + largeurPastille + 14 * u,
    y + hauteurPastille / 2,
    19 * u,
  )
  y += hauteurPastille + 24 * u

  /* Ce qui a été mis aux voix en titre, le reste de l'intitulé dessous. */
  const objet = objetDuVote(data)
  const tailleTitre = 40 * u
  police(ctx, 700, tailleTitre)
  ctx.fillStyle = p.encre
  y += paragraphe(ctx, objet.tete, G, y, colonne, lignesTitre, tailleTitre * 1.2)
  if (objet.suite) {
    const tailleSujet = 22 * u
    police(ctx, 400, tailleSujet)
    ctx.fillStyle = p.encreDouce
    y += 10 * u
    y += paragraphe(ctx, objet.suite, G, y, colonne, lignesSujet, tailleSujet * 1.4)
  }
  y += 30 * u

  /* Synthèse officielle, en chiffres lisibles de loin. */
  const s = data.synthesis
  const chiffres: [string, number, string][] = [
    ['Pour', s.tally.votes_for, p.pour],
    ['Contre', s.tally.votes_against, p.contre],
    ['Abstentions', s.tally.abstentions, p.abstention],
    ['Non-votants', s.tally.not_voting, p.encreDouce],
  ]
  const tailleChiffre = 56 * u
  chiffres.forEach(([libelle, valeur, couleur], i) => {
    const x = G + (colonne / chiffres.length) * i
    police(ctx, 600, tailleChiffre)
    ctx.fillStyle = couleur
    ctx.textBaseline = 'top'
    ctx.fillText(String(valeur), x, y)
    police(ctx, 400, 18 * u)
    ctx.fillStyle = p.encrePale
    ctx.fillText(libelle, x, y + tailleChiffre + 6 * u)
  })
  y += tailleChiffre + 6 * u + 18 * u + 22 * u

  barre(ctx, p, s.tally, G, y, colonne, 12 * u)
  y += 12 * u + 14 * u
  police(ctx, 400, 18 * u)
  ctx.fillStyle = p.encrePale
  ctx.textBaseline = 'top'
  ctx.fillText(
    `${s.voters} votants · ${s.expressed} suffrages exprimés · ${s.required} requis`,
    G,
    y,
  )
  y += 18 * u + 32 * u

  /* Pied : la source et l'adresse de la page, sur chaque image. */
  const taillePied = 18 * u
  const hauteurPied = taillePied + 28 * u
  const hautPied = bas - hauteurPied
  ctx.fillStyle = p.trait
  ctx.fillRect(G, hautPied, D - G, Math.max(1, u))
  const lien = options.lien.replace(/^https?:\/\//, '')
  // Le numéro est déjà en en-tête : s'il ne tient pas à côté de l'adresse,
  // la source se raccourcit plutôt que de la chevaucher.
  let source = `Source : Assemblée nationale, scrutin public n° ${data.number}`
  police(ctx, 600, taillePied)
  const largeurLien = ctx.measureText(lien).width
  police(ctx, 400, taillePied)
  if (ctx.measureText(source).width + largeurLien + 24 * u > D - G) {
    source = 'Source : Assemblée nationale'
  }
  morceaux(
    ctx,
    [{ texte: source, couleur: p.encrePale }],
    G,
    bas - taillePied / 2,
    taillePied,
  )
  morceaux(ctx, [{ texte: lien, couleur: p.accent, poids: 600 }], D, bas - taillePied / 2, taillePied, 'droite')

  /* Visuel, dans l'espace restant. */
  const cadre: Cadre = paysage
    ? {
        x: coupure + 24 * u,
        y: haut + hauteurLogo + 34 * u,
        largeur: D - (coupure + 24 * u),
        hauteur: hautPied - 28 * u - (haut + hauteurLogo + 34 * u),
      }
    : { x: G, y, largeur: D - G, hauteur: hautPied - 28 * u - y }
  if (cadre.hauteur < 80 * u) return

  const avecSieges = data.groups.some((g) => g.votes.some((v) => v.seat !== null))
  if (options.visuel === 'hemicycle' && avecSieges) {
    hemicycle(ctx, p, options.theme, data, cadre, u)
  } else {
    groupes(ctx, p, data, cadre, u)
  }
}

/* ------------------------------------------------------------------ story */

const STORY = GABARITS.story
const STORY_G = STORY.marges.gauche
const STORY_D = STORY.largeur - STORY.marges.droite
const STORY_HAUT = STORY.marges.haut
const STORY_BAS = STORY.hauteur - STORY.marges.bas
const STORY_LARGEUR = STORY_D - STORY_G

function sansAccents(texte: string) {
  return texte
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
}

/**
 * L'amendement voté, si son auteur est bien celui de l'intitulé officiel. Le
 * rattachement par numéro peut se tromper (n° 9758 : « amendement n° 1 du
 * Gouvernement » rattaché à l'amendement I-1 d'un député) : plutôt aucun exposé
 * que celui d'un autre. Le Gouvernement n'a pas de député auteur, la source
 * laisse alors le nom vide.
 */
function amendementDuVote(data: ScrutinDetailDto): CitedAmendmentDto | null {
  const amendement = data.cited[0]?.amendment
  if (!amendement?.summary) return null
  const intitule = sansAccents(data.subject)
  const auteur = sansAccents(amendement.author_name ?? '')
  if (intitule.includes('du gouvernement')) {
    return !auteur || auteur.includes('gouvernement') ? amendement : null
  }
  const famille = auteur.split(/\s+/).at(-1)
  return famille && intitule.includes(famille) ? amendement : null
}

/** Seconde image d'une story : seulement s'il y a un contenu réel à montrer. */
export function aUnContenuDeStory(data: ScrutinDetailDto) {
  return amendementDuVote(data) !== null
}

/** Écrit un texte à la plus grande taille qui tient en `lignes` et renvoie le y suivant. */
function texteAjuste(
  ctx: CanvasRenderingContext2D,
  texte: string,
  y: number,
  style: {
    poids: number
    max: number
    min: number
    lignes: number
    couleur: string
    interligne: number
  },
) {
  let taille = style.max
  police(ctx, style.poids, taille)
  while (
    taille > style.min &&
    couperLignes(ctx, texte, STORY_LARGEUR, Infinity).length > style.lignes
  ) {
    taille -= 2
    police(ctx, style.poids, taille)
  }
  ctx.fillStyle = style.couleur
  return (
    y +
    paragraphe(ctx, texte, STORY_G, y, STORY_LARGEUR, style.lignes, taille * style.interligne)
  )
}

function enteteStory(ctx: CanvasRenderingContext2D, data: ScrutinDetailDto) {
  const hauteurLogo = 52
  const milieu = STORY_HAUT + hauteurLogo / 2
  const largeurLogo = logo(ctx, STORY_G, STORY_HAUT, hauteurLogo, true)
  morceaux(
    ctx,
    [
      { texte: 'QuiVote', couleur: '#ffffff', poids: 600 },
      { texte: 'Quoi', couleur: '#ffffff', poids: 600 },
    ],
    STORY_G + largeurLogo + 16,
    milieu,
    38,
  )
  morceaux(
    ctx,
    [{ texte: formatDate(data.date), couleur: '#ffffff', poids: 600 }],
    STORY_D,
    milieu,
    32,
    'droite',
  )
}

function piedStory(
  ctx: CanvasRenderingContext2D,
  data: ScrutinDetailDto,
  lien: string,
  couleur: string,
) {
  const adresse = lien.replace(/^https?:\/\//, '')
  let taille = 36
  police(ctx, 700, taille)
  while (ctx.measureText(adresse).width > STORY_LARGEUR && taille > 24) {
    taille -= 1
    police(ctx, 700, taille)
  }
  morceaux(ctx, [{ texte: adresse, couleur, poids: 700 }], STORY_G, STORY_BAS - 62, taille)
  morceaux(
    ctx,
    [{ texte: `Source : Assemblée nationale, scrutin n° ${data.number}`, couleur }],
    STORY_G,
    STORY_BAS - 16,
    30,
  )
}

/** Titre du vote et reste de l'intitulé. Renvoie le y suivant. */
function objetStory(
  ctx: CanvasRenderingContext2D,
  objet: ReturnType<typeof objetDuVote>,
  yDepart: number,
  styles: { couleur: string; couleurSuite: string; titre: number; suite: number; lignesSuite: number },
) {
  const y = objet.court
    ? texteAjuste(ctx, objet.tete, yDepart, {
        poids: 700,
        max: styles.titre,
        min: 56,
        lignes: 1,
        couleur: styles.couleur,
        interligne: 1.16,
      })
    : texteAjuste(ctx, objet.tete, yDepart, {
        poids: 700,
        max: Math.round(styles.titre * 0.72),
        min: 44,
        lignes: 5,
        couleur: styles.couleur,
        interligne: 1.16,
      })
  if (!objet.suite) return y
  return texteAjuste(ctx, objet.suite, y + 16, {
    poids: 400,
    max: styles.suite,
    min: 28,
    lignes: styles.lignesSuite,
    couleur: styles.couleurSuite,
    interligne: 1.28,
  })
}

/** Première image d'une story : le vote, l'hémicycle bord à bord sur fond sombre. */
function dessinerStoryVote(canvas: HTMLCanvasElement, data: ScrutinDetailDto, lien: string) {
  const p = PALETTES.sombre
  canvas.width = STORY.largeur
  canvas.height = STORY.hauteur
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  ctx.fillStyle = p.fond
  ctx.fillRect(0, 0, STORY.largeur, STORY.hauteur)
  enteteStory(ctx, data)

  const y = objetStory(ctx, objetDuVote(data), STORY_HAUT + 96, {
    couleur: p.encre,
    couleurSuite: p.encreDouce,
    titre: 100,
    suite: 32,
    lignesSuite: 5,
  })

  const yChiffres = STORY_BAS - 90 - 136
  const yIssue = yChiffres - 128

  const avecSieges = data.groups.some((g) => g.votes.some((v) => v.seat !== null))
  if (avecSieges) {
    const bord = 20
    hemicycle(
      ctx,
      p,
      'sombre',
      data,
      { x: bord, y: y + 16, largeur: STORY.largeur - bord * 2, hauteur: yIssue - y - 32 },
      1.6,
    )
  } else {
    groupes(
      ctx,
      p,
      data,
      { x: STORY_G, y: y + 16, largeur: STORY_LARGEUR, hauteur: yIssue - y - 32 },
      1.3,
    )
  }

  morceaux(
    ctx,
    [
      {
        texte: majuscule(data.outcome_code),
        couleur: data.outcome_code === 'adopté' ? p.pour : p.contre,
        poids: 700,
      },
    ],
    STORY_G - 4,
    yIssue + 54,
    108,
  )

  const t = data.synthesis.tally
  const chiffres: [string, number, string][] = [
    ['Pour', t.votes_for, p.pour],
    ['Contre', t.votes_against, p.contre],
    ['Abst.', t.abstentions, p.abstention],
    ['Non-votants', t.not_voting, p.encreDouce],
  ]
  chiffres.forEach(([libelle, valeur, couleur], i) => {
    const x = STORY_G + (i * STORY_LARGEUR) / chiffres.length
    morceaux(ctx, [{ texte: String(valeur), couleur, poids: 700 }], x, yChiffres + 40, 80)
    morceaux(ctx, [{ texte: libelle, couleur: p.encreDouce, poids: 500 }], x, yChiffres + 104, 30)
  })

  piedStory(ctx, data, lien, p.encre)
}

/**
 * Début de l'exposé qui tient dans `maxLignes`, sans choix éditorial : on prend
 * depuis le début et on coupe à la fin d'une phrase. `[…]` dit la coupure.
 */
function extraitExpose(
  ctx: CanvasRenderingContext2D,
  expose: string,
  largeur: number,
  maxLignes: number,
) {
  const blocs: string[][] = []
  let restant = maxLignes
  for (const paragraphe of expose.split(/\n+/).map((t) => t.trim()).filter(Boolean)) {
    const lignes = couperLignes(ctx, paragraphe, largeur, Infinity)
    if (lignes.length <= restant) {
      blocs.push(lignes)
      // L'espace entre deux paragraphes vaut une demi-ligne.
      restant -= lignes.length + 0.5
      continue
    }
    let garde = ''
    for (const phrase of paragraphe.split(/(?<=[.!?…])\s+/)) {
      const essai = garde ? `${garde} ${phrase}` : phrase
      if (couperLignes(ctx, `${essai} […]`, largeur, Infinity).length > Math.floor(restant)) break
      garde = essai
    }
    if (garde) {
      blocs.push(couperLignes(ctx, `${garde} […]`, largeur, Infinity))
    } else if (blocs.length > 0) {
      const dernier = blocs[blocs.length - 1].join(' ')
      blocs[blocs.length - 1] = couperLignes(ctx, `${dernier} […]`, largeur, Infinity)
    } else {
      blocs.push(couperLignes(ctx, paragraphe, largeur, Math.max(1, Math.floor(restant))))
    }
    break
  }
  return blocs
}

/**
 * Seconde image d'une story : l'exposé, sur fond à la couleur de l'issue. La
 * pastille redit le résultat, la couleur seule ne suffit pas à le dire. L'exposé
 * est l'argumentaire de son auteur, non un constat : l'image le dit (README.md §6).
 * Renvoie `false` sans rien dessiner s'il n'y a pas de contenu à montrer.
 */
export function dessinerContenuStory(
  canvas: HTMLCanvasElement,
  data: ScrutinDetailDto,
  lien: string,
) {
  const amendement = amendementDuVote(data)
  if (!amendement?.summary) return false
  canvas.width = STORY.largeur
  canvas.height = STORY.hauteur
  const ctx = canvas.getContext('2d')
  if (!ctx) return false

  const adopte = data.outcome_code === 'adopté'
  const fond = adopte ? PALETTES.clair.pour : PALETTES.clair.contre
  const blanc = '#ffffff'
  const blancDoux = 'rgba(255, 255, 255, 0.88)'

  ctx.fillStyle = fond
  ctx.fillRect(0, 0, STORY.largeur, STORY.hauteur)
  enteteStory(ctx, data)

  // Le reste de l'intitulé est déjà sur la première image : la place va à l'exposé.
  let y = objetStory(ctx, { ...objetDuVote(data), suite: '' }, STORY_HAUT + 96, {
    couleur: blanc,
    couleurSuite: blanc,
    titre: 84,
    suite: 0,
    lignesSuite: 0,
  })
  y += 18

  const resultat = majuscule(data.outcome_code)
  police(ctx, 700, 40)
  ctx.fillStyle = blanc
  ctx.beginPath()
  ctx.roundRect(STORY_G, y, ctx.measureText(resultat).width + 44, 72, 14)
  ctx.fill()
  morceaux(ctx, [{ texte: resultat, couleur: fond, poids: 700 }], STORY_G + 22, y + 37, 40)
  y += 72 + 40

  // La couleur du groupe reste cantonnée à sa pastille, cerclée pour rester
  // visible sur le fond (README.md §6).
  ctx.fillStyle = amendement.author_group_color ?? PALETTES.clair.traitFort
  ctx.strokeStyle = blanc
  ctx.lineWidth = 3
  ctx.beginPath()
  ctx.arc(STORY_G + 15, y + 22, 15, 0, Math.PI * 2)
  ctx.fill()
  ctx.stroke()

  const groupe = amendement.author_group_abbrev ? ` (${amendement.author_group_abbrev})` : ''
  const nb = amendement.cosignatory_count
  const cosignataires = nb > 0 ? ` + ${nb} cosignataire${nb > 1 ? 's' : ''}` : ''
  const auteur = `${amendement.author_name ?? 'Gouvernement'}${groupe}${cosignataires}`
  let tailleAuteur = 36
  police(ctx, 600, tailleAuteur)
  while (ctx.measureText(auteur).width > STORY_LARGEUR - 44 && tailleAuteur > 24) {
    tailleAuteur -= 1
    police(ctx, 600, tailleAuteur)
  }
  morceaux(ctx, [{ texte: auteur, couleur: blanc, poids: 600 }], STORY_G + 44, y + 22, tailleAuteur)
  y += 62

  const mention = amendement.author_name
    ? 'Exposé rédigé par l’auteur de l’amendement'
    : 'Exposé rédigé par le Gouvernement'
  morceaux(ctx, [{ texte: mention, couleur: blancDoux }], STORY_G, y + 16, 30)
  y += 58

  /* Le début de l'exposé, présenté en citation. */
  const retrait = 28
  const taille = 42
  const interligne = taille * 1.34
  const finTexte = STORY_BAS - 130
  police(ctx, 400, taille)
  // Une ligne de réserve : le `[…]` ajouté à un paragraphe peut en créer une.
  const blocs = extraitExpose(
    ctx,
    amendement.summary,
    STORY_LARGEUR - retrait,
    Math.floor((finTexte - y) / interligne) - 1,
  )
  const debutCitation = y
  ctx.fillStyle = blanc
  ctx.textBaseline = 'top'
  blocs.forEach((lignes, i) => {
    if (i > 0) y += interligne / 2
    lignes.forEach((ligne) => {
      ctx.fillText(ligne, STORY_G + retrait, y)
      y += interligne
    })
  })
  ctx.fillStyle = 'rgba(255, 255, 255, 0.55)'
  ctx.beginPath()
  ctx.roundRect(STORY_G, debutCitation + 4, 6, y - debutCitation - 14, 3)
  ctx.fill()

  piedStory(ctx, data, lien, blanc)
  return true
}

/** Ligne d'un groupe dans le texte d'accompagnement : les chiffres de l'image, en toutes lettres. */
function ligneGroupe(groupe: GroupBreakdownDto) {
  const t = groupe.tally
  const chiffres = [
    `${t.votes_for} pour`,
    `${t.votes_against} contre`,
    `${t.abstentions} abstention${t.abstentions > 1 ? 's' : ''}`,
    ...(t.not_voting > 0 ? [`${t.not_voting} non-votant${t.not_voting > 1 ? 's' : ''}`] : []),
  ]
  return `${nomGroupe(groupe)} : ${chiffres.join(', ')}`
}

/**
 * Texte d'accompagnement : les nombres, sans adverbe, le vote de chaque groupe
 * et le lien. Là où le détail dépasse la limite du réseau, il cède la place au
 * total : tronquer la liste ferait disparaître des groupes (README.md §2).
 */
export function texteDePartage(data: ScrutinDetailDto, lien: string, reseau: ReseauImage) {
  const t = data.synthesis.tally
  const titre = majuscule(data.subject)
  const resume =
    `Scrutin n° ${data.number} du ${formatDate(data.date)} — ${majuscule(data.outcome_code)} : ` +
    `${t.votes_for} pour, ${t.votes_against} contre, ${t.abstentions} abstention${t.abstentions > 1 ? 's' : ''}.\n` +
    titre
  const court = `${resume}\n${lien}`
  if (data.groups.length === 0) return court

  const reconstitue = data.groups.some((g) => g.origin === 'reconstructed')
  const complet = [
    resume,
    '',
    'Vote par groupe :',
    ...data.groups.map(ligneGroupe),
    ...(reconstitue ? ['', NOTE_RECONSTITUE] : []),
    '',
    lien,
  ].join('\n')
  const limite = RESEAUX[reseau].limiteTexte
  return limite !== undefined && complet.length > limite ? court : complet
}
