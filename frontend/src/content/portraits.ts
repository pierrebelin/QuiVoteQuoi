import type { Logo } from './logos'

/**
 * Portraits des candidats, servis depuis `public/candidats/`.
 *
 * Image principale de la fiche Wikidata de la personne, sur Wikimedia Commons,
 * réduite à 240 px et réencodée en JPEG. Licences libres : la page « À propos »
 * en tire la mention de crédit. L'année est celle de la prise de vue — un
 * portrait ancien reste daté plutôt que présenté comme actuel.
 *
 * Indexé par identifiant de candidature. Sans portrait libre, la page affiche
 * une silhouette neutre : jamais une image d'une autre source.
 */
export interface Portrait extends Logo {
  year: string | null
}

export const PORTRAITS: Record<string, Portrait> = {
  'edouard-philippe': {
    src: '/candidats/edouard-philippe.jpg',
    credit: { file: 'Édouard Philippe 2019 (cropped).jpg', page: 'https://commons.wikimedia.org/wiki/File:%C3%89douard_Philippe_2019_(cropped).jpg', license: 'CC BY 2.0', author: 'Jacques Paquier' },
    year: '2019',
  },
  'nicolas-dupont-aignan': {
    src: '/candidats/nicolas-dupont-aignan.jpg',
    credit: { file: 'Nicolas Dupont-Aignan, homme politique français.jpg', page: 'https://commons.wikimedia.org/wiki/File:Nicolas_Dupont-Aignan,_homme_politique_fran%C3%A7ais.jpg', license: 'CC BY-SA 4.0', author: 'Debout la France' },
    year: '2019',
  },
  'delphine-batho': {
    src: '/candidats/delphine-batho.jpg',
    credit: { file: 'Delphine Batho (cropped).png', page: 'https://commons.wikimedia.org/wiki/File:Delphine_Batho_(cropped).png', license: 'CC BY-SA 4.0', author: 'DeuxSevres79' },
    year: '2018',
  },
  'nathalie-arthaud': {
    src: '/candidats/nathalie-arthaud.jpg',
    credit: { file: 'Nathalie Arthaud (LO) 19-05-2024.jpg', page: 'https://commons.wikimedia.org/wiki/File:Nathalie_Arthaud_(LO)_19-05-2024.jpg', license: 'CC BY-SA 4.0', author: 'Kokluts' },
    year: '2024',
  },
  'marine-tondelier': {
    src: '/candidats/marine-tondelier.jpg',
    credit: { file: 'Huma2023MarineTondelier.jpg', page: 'https://commons.wikimedia.org/wiki/File:Huma2023MarineTondelier.jpg', license: 'CC BY 4.0', author: 'Selbymay' },
    year: '2023',
  },
  'juan-branco': {
    src: '/candidats/juan-branco.jpg',
    credit: { file: 'Juan Branco Pick.jpg', page: 'https://commons.wikimedia.org/wiki/File:Juan_Branco_Pick.jpg', license: 'CC BY 4.0', author: 'UPOP Marseille' },
    year: '2024',
  },
  'francois-ruffin': {
    src: '/candidats/francois-ruffin.jpg',
    credit: { file: 'François Ruffin (cropped).jpg', page: 'https://commons.wikimedia.org/wiki/File:Fran%C3%A7ois_Ruffin_(cropped).jpg', license: 'CC BY-SA 4.0', author: 'Thinkerview' },
    year: '2022',
  },
  'bruno-retailleau': {
    src: '/candidats/bruno-retailleau.jpg',
    credit: { file: 'Bruno Retailleau - Ministre de l\'Intérieur français (cropped).jpg', page: 'https://commons.wikimedia.org/wiki/File:Bruno_Retailleau_-_Ministre_de_l%27Int%C3%A9rieur_fran%C3%A7ais_(cropped).jpg', license: 'CC BY 4.0', author: 'Anthonymontardyfr' },
    year: '2025',
  },
  'francois-asselineau': {
    src: '/candidats/francois-asselineau.jpg',
    credit: { file: 'François ASSELINEAU.jpg', page: 'https://commons.wikimedia.org/wiki/File:Fran%C3%A7ois_ASSELINEAU.jpg', license: 'CC BY-SA 4.0', author: 'Union Populaire Républicaine' },
    year: '2014',
  },
  'david-lisnard': {
    src: '/candidats/david-lisnard.jpg',
    credit: { file: 'David Lisnard en 2026.jpg', page: 'https://commons.wikimedia.org/wiki/File:David_Lisnard_en_2026.jpg', license: 'CC BY-SA 4.0', author: 'Grégor Sauvagère' },
    year: '2026',
  },
  'clara-egger': {
    src: '/candidats/clara-egger.jpg',
    credit: { file: 'ClaraEgger 2021-12-29 18-16-03.jpg', page: 'https://commons.wikimedia.org/wiki/File:ClaraEgger_2021-12-29_18-16-03.jpg', license: 'CC BY-SA 4.0', author: 'Thomas Binet' },
    year: '2021',
  },
  'jean-luc-melenchon': {
    src: '/candidats/jean-luc-melenchon.jpg',
    credit: { file: 'Mélenchon 2027 - 55262255527 (cropped 3-4).png', page: 'https://commons.wikimedia.org/wiki/File:M%C3%A9lenchon_2027_-_55262255527_(cropped_3-4).png', license: 'CC BY 4.0', author: 'jlm2017.fr' },
    year: '2026',
  },
  'florian-philippot': {
    src: '/candidats/florian-philippot.jpg',
    credit: { file: 'Florian Philippot.jpg', page: 'https://commons.wikimedia.org/wiki/File:Florian_Philippot.jpg', license: 'CC BY 4.0', author: 'Films et documentaires' },
    year: '2024',
  },
  'gabriel-attal': {
    src: '/candidats/gabriel-attal.jpg',
    credit: { file: 'Gabriel Attal 2025 (close crop).jpg', page: 'https://commons.wikimedia.org/wiki/File:Gabriel_Attal_2025_(close_crop).jpg', license: 'CC BY 4.0', author: 'Ismail Aissoub' },
    year: '2025',
  },
  'anasse-kazib': {
    src: '/candidats/anasse-kazib.jpg',
    credit: { file: 'Anasse Kazib, décembre 2021.jpg', page: 'https://commons.wikimedia.org/wiki/File:Anasse_Kazib,_d%C3%A9cembre_2021.jpg', license: 'CC BY-SA 4.0', author: 'PabloMorao27' },
    year: '2021',
  },
  'karim-bouamrane': {
    src: '/candidats/karim-bouamrane.jpg',
    credit: { file: 'Karim Bouamrane en 2026.jpg', page: 'https://commons.wikimedia.org/wiki/File:Karim_Bouamrane_en_2026.jpg', license: 'CC BY 4.0', author: 'Oumma' },
    year: '2026',
  },
  'marine-le-pen': {
    src: '/candidats/marine-le-pen.jpg',
    credit: { file: 'Марин Ле Пен (28-01-2022).jpg', page: 'https://commons.wikimedia.org/wiki/File:%D0%9C%D0%B0%D1%80%D0%B8%D0%BD_%D0%9B%D0%B5_%D0%9F%D0%B5%D0%BD_(28-01-2022).jpg', license: 'Domaine public', author: 'VOX España' },
    year: '2022',
  },
  'francis-lalanne': {
    src: '/candidats/francis-lalanne.jpg',
    credit: { file: 'Francis Lalanne.png', page: 'https://commons.wikimedia.org/wiki/File:Francis_Lalanne.png', license: 'Licence Art Libre', author: 'Olivier Roller, In Libro Veritas' },
    year: '2011',
  },
  'raphael-glucksmann': {
    src: '/candidats/raphael-glucksmann.jpg',
    credit: { file: '2024-03-03 - Congrès du PSE- 105 - Raphael Glucksmann.jpg', page: 'https://commons.wikimedia.org/wiki/File:2024-03-03_-_Congr%C3%A8s_du_PSE-_105_-_Raphael_Glucksmann.jpg', license: 'Domaine public', author: 'Mathieu Delmestre' },
    year: '2024',
  },
  'fabien-roussel': {
    src: '/candidats/fabien-roussel.jpg',
    credit: { file: 'Roussel Fabien 1.jpg', page: 'https://commons.wikimedia.org/wiki/File:Roussel_Fabien_1.jpg', license: 'CC BY-SA 4.0', author: 'Zouhair NAKARA' },
    year: '2018',
  },
  'eric-zemmour': {
    src: '/candidats/eric-zemmour.jpg',
    credit: { file: 'Portrait d\'Éric Zemmour, avril 2022.jpg', page: 'https://commons.wikimedia.org/wiki/File:Portrait_d%27%C3%89ric_Zemmour,_avril_2022.jpg', license: 'CC0', author: 'Anh De France' },
    year: '2022',
  },
  'olivier-becht': {
    src: '/candidats/olivier-becht.jpg',
    credit: { file: 'Olivier BECHT.jpg', page: 'https://commons.wikimedia.org/wiki/File:Olivier_BECHT.jpg', license: 'CC BY 3.0', author: 'Werner STEINKE' },
    year: null,
  },
  'natacha-polony': {
    src: '/candidats/natacha-polony.jpg',
    credit: { file: 'Natacha Polony Cannes 2015.jpg', page: 'https://commons.wikimedia.org/wiki/File:Natacha_Polony_Cannes_2015.jpg', license: 'CC BY-SA 3.0', author: 'Georges Biard' },
    year: '2015',
  },
}
