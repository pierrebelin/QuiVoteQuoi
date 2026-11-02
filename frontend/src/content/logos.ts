/**
 * Logos officiels des groupes parlementaires et des partis, servis depuis
 * `public/logos/`.
 *
 * Tous viennent de Wikimedia Commons, sous la licence de leur page : domaine
 * public, CC0, ou CC BY / BY-SA — ces deux dernières exigent la mention de
 * crédit que la page « À propos » tire de ce fichier. Un logo périmé (ancien nom
 * de groupe) est pire que pas de logo : il est retiré, et le sigle le remplace.
 *
 * Un groupe n'est pas un parti : un groupe porte son propre logo quand il en
 * publie un, sinon celui du parti dont il reprend le nom (SOC, HOR, UDDPLR).
 * LFI-NFP et DEM prennent celui de leur parti : les seuls logos de groupe
 * disponibles portent un nom abandonné (NUPES, « Mouvement démocrate et
 * apparentés »). NI n'a pas de logo, par construction.
 */
export interface Logo {
  src: string
  credit: {
    file: string
    page: string
    license: string
    author: string
  }
}

function commons(src: string, file: string, license: string, author: string): Logo {
  return {
    src: `/logos/${src}`,
    credit: {
      file,
      page: `https://commons.wikimedia.org/wiki/File:${encodeURIComponent(file.replaceAll(' ', '_'))}`,
      license,
      author,
    },
  }
}

const RENAISSANCE = commons('renaissance.svg', 'Renaissance parti logo.svg', 'Domaine public', 'Renaissance')
const HORIZONS = commons('horizons.svg', 'Logo Horizons.svg', 'Domaine public', 'Horizons')
const LFI = commons('lfi.svg', 'Logo France Insoumise - Cropped.svg', 'CC0', 'La France insoumise')
const LES_REPUBLICAINS = commons(
  'les-republicains.svg',
  'Les Républicains - logo (France, 2023) (cropped).svg',
  'Domaine public',
  'Les Républicains',
)
const LES_ECOLOGISTES = commons('les-ecologistes.png', 'Logo Les Écologistes (France).png', 'Domaine public', 'Les Écologistes')
const PLACE_PUBLIQUE = commons('place-publique.svg', 'Logo Place publique.svg', 'Domaine public', 'Place publique')
const MODEM = commons('modem.svg', 'MoDem logo 2019 - Cropped.svg', 'Domaine public', 'Mouvement démocrate')

/** Par sigle de groupe, tel que l'Assemblée le publie. */
export const GROUP_LOGOS: Record<string, Logo> = {
  RN: commons('groupe-rn.svg', 'Groupe Rassemblement national logo.svg', 'Domaine public', 'Groupe Rassemblement national'),
  EPR: commons('groupe-epr.png', 'Groupe EPR.png', 'CC BY 4.0', 'Ensemble pour la République'),
  'LFI-NFP': LFI,
  SOC: commons('parti-socialiste.svg', 'Le Parti socialiste wordmark.svg', 'Domaine public', 'Le Parti socialiste'),
  DR: commons(
    'groupe-dr.png',
    'La Droite Républicaine logo 2024.png',
    'CC BY-SA 4.0',
    'La Droite Républicaine – Députés & Apparentés',
  ),
  ECOS: commons('groupe-ecos.webp', 'Logo-Groupe écologiste et Social-AN.webp', 'CC BY 3.0', 'Auteur inconnu'),
  DEM: MODEM,
  HOR: HORIZONS,
  LIOT: commons('groupe-liot.png', 'LIOT Group.png', 'CC BY-SA 3.0', 'Groupe Libertés, indépendants, outre-mer et territoires'),
  GDR: commons('groupe-gdr.svg', 'Logo GDR.svg', 'CC BY-SA 4.0', 'Groupe de la Gauche démocrate et républicaine'),
  UDDPLR: commons('udr.svg', 'UDR logo.svg', 'Domaine public', 'Union des droites pour la République'),
}

/**
 * Par nom d'organisation, tel que la candidature le source. Les partis n'ont
 * pas de couleur publiée par l'Assemblée : la leur est la teinte dominante de
 * leur logo.
 */
export const PARTIES: Record<string, { logo: Logo; color: string }> = {
  Renaissance: { logo: RENAISSANCE, color: '#1a203b' },
  Horizons: { logo: HORIZONS, color: '#0000ff' },
  'La France insoumise': { logo: LFI, color: '#c84529' },
  'Les Républicains': { logo: LES_REPUBLICAINS, color: '#0045ae' },
  'Les Écologistes': { logo: LES_ECOLOGISTES, color: '#3a9a3c' },
  'Place publique': { logo: PLACE_PUBLIQUE, color: '#fff100' },
  'Rassemblement National': {
    logo: commons('rassemblement-national.svg', 'Logo Rassemblement National (cropped).svg', 'Domaine public', 'Rassemblement national'),
    color: '#1b4677',
  },
  'Parti communiste français': {
    logo: commons('pcf.svg', 'Logo – Parti communiste français (2018).svg', 'Domaine public', 'Parti communiste français'),
    color: '#ff5252',
  },
  Reconquête: {
    logo: commons('reconquete.svg', 'Logo of the Reconquest (political party).svg', 'Domaine public', 'Reconquête'),
    color: '#0b0b66',
  },
  'Lutte ouvrière': {
    logo: commons('lutte-ouvriere.svg', 'Logo Lutte Ouvrière.svg', 'Domaine public', 'Lutte ouvrière'),
    color: '#ed1b24',
  },
  'Union populaire républicaine (UPR)': {
    logo: commons('upr.svg', 'Logo Union Populaire Républicaine.svg', 'Domaine public', 'Union populaire républicaine'),
    color: '#004389',
  },
  'NPA-Révolutionnaires': {
    logo: commons('npa-revolutionnaires.svg', 'Logo-NPA Revo.svg', 'CC0', 'NPA-Révolutionnaires'),
    color: '#cd071e',
  },
  'Debout la France': {
    logo: commons('debout-la-france.png', 'Debout la France logo (2022).png', 'CC BY-SA 4.0', 'Debout la France'),
    color: '#1b5fae',
  },
  'Solution démocratique': {
    logo: commons(
      'solution-democratique.png',
      'Logo du parti politique français Solution Démocratique.png',
      'CC BY 4.0',
      'Solution démocratique',
    ),
    color: '#00c8d7',
  },
  Équinoxe: { logo: commons('equinoxe.png', 'Équinoxe Logo.png', 'CC0', 'Équinoxe'), color: '#f2c200' },
}

/** Chaque fichier une fois, pour la mention de crédit. */
export const ALL_LOGOS: Logo[] = [
  ...new Map(
    [...Object.values(GROUP_LOGOS), ...Object.values(PARTIES).map((party) => party.logo)].map((logo) => [logo.src, logo]),
  ).values(),
]
