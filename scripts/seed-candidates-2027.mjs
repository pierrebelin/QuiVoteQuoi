#!/usr/bin/env node

/**
 * Peuple le registre des candidatures presidentielles 2027 dont la
 * declaration est documentee par une source primaire.
 *
 * Programmes : seul le site officiel du candidat fait foi (README.md §8.2).
 * programUrl et la source de chaque extrait doivent etre sur ce site, ou dans
 * officialDocuments : un document heberge ailleurs mais lie directement depuis
 * le site. Le script echoue sinon. Candidat sans programme verifiable : rien
 * plutot qu'une source tierce.
 *
 * Les extraits sont reproduits tels quels. Leur presence mot pour mot dans leur
 * source n'est pas controlee ici mais par scripts/check-candidate-sources.mjs,
 * a relancer apres toute modification des extraits.
 * Aucun groupe parlementaire n'est ajoute ici : ce lien exige sa propre source
 * explicite (README.md §3.3 et §8.2).
 *
 * Usage :
 *   set -a; source .env; set +a
 *   node scripts/seed-candidates-2027.mjs
 *
 * Depuis le poste de developpement (sans client psql local) :
 *   node scripts/seed-candidates-2027.mjs --docker
 *
 * Pour examiner la transaction sans ecrire :
 *   node scripts/seed-candidates-2027.mjs --dry-run
 *
 * Pour lire le registre en JSON (utilise par le controle) :
 *   node scripts/seed-candidates-2027.mjs --json
 */
import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const options = new Set(process.argv.slice(2))
if (![...options].every((option) => ['--dry-run', '--docker', '--json'].includes(option))) {
  throw new Error('Usage: node scripts/seed-candidates-2027.mjs [--dry-run] [--docker] [--json]')
}
const dryRun = options.has('--dry-run')
const useDocker = options.has('--docker')

const candidates = [
  {
    id: 'edouard-philippe',
    displayName: 'Édouard Philippe',
    declaredOn: '2024-09-03',
    declarationSourceUrl: 'https://www.lepoint.fr/politique/exclusif-edouard-philippe-je-serai-candidat-a-la-prochaine-election-presidentielle-03-09-2024-2569359_20.php',
    declarationSourceLabel: 'Le Point — « Je serai candidat à la prochaine élection présidentielle »',
    officialSiteUrl: 'https://www.edouardphilippe.fr/',
    programUrl: 'https://www.edouardphilippe.fr/',
    organization: {
      id: 'horizons',
      label: 'Horizons',
      officialUrl: 'https://horizonsleparti.fr/',
      sourceUrl: 'https://horizonsleparti.fr/donner/',
      sourceLabel: 'Horizons — « Soutenez Édouard Philippe en donnant à Horizons »',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Inscrire dans la Constitution une règle d\'or de maîtrise des déficits via un référendum, et ramener le déficit de plus de 5% à 2% du PIB à la fin du quinquennat.',
        sourceUrl: 'https://www.edouardphilippe.fr/priorites/pour-une-france-plus-prospere',
        sourceLabel: 'Avec Édouard 2027 — Pour une France plus prospère',
      },
      {
        familyCode: 'pouvoir-achat',
        excerpt: 'Sceller un moratoire normatif et fiscal : pas de nouvelle norme et de nouvel impôt sur le quinquennat.',
        sourceUrl: 'https://www.edouardphilippe.fr/priorites/pour-une-france-plus-prospere',
        sourceLabel: 'Avec Édouard 2027 — Pour une France plus prospère',
      },
      {
        familyCode: 'international-defense',
        excerpt: 'Massifier notre production et nos achats de drones tactiques, sur le modèle du programme américain de 300 000 drones.',
        sourceUrl: 'https://www.edouardphilippe.fr/priorites/pour-une-france-plus-sure',
        sourceLabel: 'Avec Édouard 2027 — Pour une France plus sûre',
      },
    ],
  },
  {
    id: 'jean-luc-melenchon',
    displayName: 'Jean-Luc Mélenchon',
    declaredOn: '2026-05-03',
    declarationSourceUrl: 'https://lafranceinsoumise.fr/2026/05/03/declaration-de-lintergroupe-de-la-france-insoumise-du-3-mai-2026/',
    declarationSourceLabel: 'La France insoumise — Déclaration de l’intergroupe du 3 mai 2026',
    officialSiteUrl: 'https://melenchon2027.fr/',
    programUrl: 'https://melenchon2027.fr/wp-content/uploads/2025/02/A5-PROGRAMME-SITE.pdf',
    organization: {
      id: 'la-france-insoumise',
      label: 'La France insoumise',
      officialUrl: 'https://lafranceinsoumise.fr/',
      sourceUrl: 'https://lafranceinsoumise.fr/2026/05/03/declaration-de-lintergroupe-de-la-france-insoumise-du-3-mai-2026/',
      sourceLabel: 'La France insoumise — Déclaration de l’intergroupe du 3 mai 2026',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Convoquer une Constituante pour passer à la 6e République.',
        sourceUrl: 'https://melenchon2027.fr/wp-content/uploads/2025/02/A5-PROGRAMME-SITE.pdf#page=11',
        sourceLabel: 'L’Avenir en commun, édition 2025 — Réunir une Assemblée constituante pour passer à la 6e République',
      },
      {
        familyCode: 'pouvoir-achat',
        excerpt: 'Rendre l’impôt sur le revenu plus progressif avec un barème à 14 tranches contre 5 aujourd’hui.',
        sourceUrl: 'https://melenchon2027.fr/wp-content/uploads/2025/02/A5-PROGRAMME-SITE.pdf#page=26',
        sourceLabel: 'L’Avenir en commun, édition 2025 — Faire la révolution fiscale',
      },
      {
        familyCode: 'environnement-energie',
        excerpt: 'Inscrire dans la Constitution le principe de la « règle verte », selon laquelle on ne prélève pas davantage à la nature que ce qu’elle est en état de reconstituer.',
        sourceUrl: 'https://melenchon2027.fr/wp-content/uploads/2025/02/A5-PROGRAMME-SITE.pdf#page=46',
        sourceLabel: 'L’Avenir en commun, édition 2025 — La bifurcation écologique pour une société de l’harmonie',
      },
      {
        familyCode: 'international-defense',
        excerpt: 'Se retirer immédiatement du commandement intégré de l’OTAN puis, par étapes, de l’organisation elle-même.',
        sourceUrl: 'https://melenchon2027.fr/wp-content/uploads/2025/02/A5-PROGRAMME-SITE.pdf#page=59',
        sourceLabel: 'L’Avenir en commun, édition 2025 — Assumer l’indépendance de la France dans le monde',
      },
    ],
  },
  {
    id: 'gabriel-attal',
    displayName: 'Gabriel Attal',
    declaredOn: '2026-05-22',
    declarationSourceUrl: 'https://attalpresident.fr/actualites/annonce-de-candidature',
    declarationSourceLabel: 'Attal Président — Annonce de candidature depuis l’Aveyron',
    officialSiteUrl: 'https://attalpresident.fr/',
    programUrl: 'https://attalpresident.fr/programme',
    organization: {
      id: 'renaissance',
      label: 'Renaissance',
      officialUrl: 'https://parti-renaissance.fr/',
      sourceUrl: 'https://attalpresident.fr/actualites/annonce-de-candidature',
      sourceLabel: 'Attal Président — Annonce de candidature depuis l’Aveyron',
    },
    proposals: [
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'L’objectif est clair : zéro déficit en 10 ans maximum.',
        sourceUrl: 'https://attalpresident.fr/programme/dette-de-letat',
        sourceLabel: 'Attal Président — Dette de l’État',
      },
      {
        familyCode: 'travail-emploi',
        excerpt: 'Simplifier notre droit, réformer le code du travail pour en faire une constitution du travail avec les grands principes et donner ensuite plus de liberté au dialogue social pour fixer l’organisation du travail.',
        sourceUrl: 'https://attalpresident.fr/programme/travail-salaires',
        sourceLabel: 'Attal Président — Travail & salaires',
      },
      {
        familyCode: 'education-culture',
        excerpt: 'Je souhaite que, dès 2027, une loi de programmation pour l’école soit votée, pilotée par un ministre de l’éducation nationale qui restera en poste cinq ans.',
        sourceUrl: 'https://attalpresident.fr/actualites/dans-le-monde-gabriel-attal-devoile-son-projet-pour-faire-notre-ecole-la-meilleure-d-europe',
        sourceLabel: 'Attal Président — Projet pour l’École',
      },
      {
        familyCode: 'international-defense',
        excerpt: 'Il propose un nouveau Livre blanc de la défense dès 2027 afin de revoir les priorités de notre stratégie militaire, ainsi qu’une revue complète des programmes pour adapter les équipements aux réalités des conflits actuels.',
        sourceUrl: 'https://attalpresident.fr/actualites/aux-rencontres-economiques-d-aix-en-provence-gabriel-attal-appelle-la-france-a-preparer-les-guerres-de-demain',
        sourceLabel: 'Attal Président — Préparer les guerres de demain',
      },
    ],
  },
  {
    id: 'marine-le-pen',
    displayName: 'Marine Le Pen',
    declaredOn: '2026-07-07',
    declarationSourceUrl: 'https://www.tf1info.fr/politique/marine-le-pen-sur-tf1-candidature-a-la-presidentielle-pourvoi-en-cassation-role-de-jordan-bardella-ce-qu-il-faut-retenir-2451897.html',
    declarationSourceLabel: 'TF1 — Déclaration au journal de 20 heures du 7 juillet 2026',
    officialSiteUrl: 'https://marinelepen.com/',
    programUrl: null,
    organization: {
      id: 'rassemblement-national',
      label: 'Rassemblement National',
      officialUrl: 'https://rassemblementnational.fr/',
      sourceUrl: 'https://rassemblementnational.fr/instances/bureau-executif',
      sourceLabel: 'Rassemblement National — Bureau exécutif (Marine Le Pen, députée)',
    },
    proposals: [],
  },
  {
    id: 'marine-tondelier',
    displayName: 'Marine Tondelier',
    declaredOn: '2025-12-08',
    declarationSourceUrl: 'https://lesecologistes.fr/posts/4DGpyusxBAU4xQPexfncsx/marine-tondelier-designee-pour-representer-les-ecologistes-a-l-election-presidentielle',
    declarationSourceLabel: 'Les Écologistes — Désignation pour l’élection présidentielle',
    officialSiteUrl: 'https://marinetondelier.fr/',
    programUrl: 'https://marinetondelier.fr/programme-2027',
    organization: {
      id: 'les-ecologistes',
      label: 'Les Écologistes',
      officialUrl: 'https://lesecologistes.fr/',
      sourceUrl: 'https://lesecologistes.fr/posts/4DGpyusxBAU4xQPexfncsx/marine-tondelier-designee-pour-representer-les-ecologistes-a-l-election-presidentielle',
      sourceLabel: 'Les Écologistes — Désignation pour l’élection présidentielle',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Nous ferons de la lutte contre la corruption une priorité, garantirons l’indépendance de la justice et des médias, et moderniserons nos institutions pour fonder la première république écologique et citoyenne, plus proche de la souveraineté populaire.',
        sourceUrl: 'https://marinetondelier.fr/s/Programme-2027.pdf#page=7',
        sourceLabel: 'Marine Tondelier — Programme 2027 (PDF)',
      },
    ],
  },
  {
    id: 'bruno-retailleau',
    displayName: 'Bruno Retailleau',
    declaredOn: '2026-02-12',
    declarationSourceUrl: 'https://www.avecretailleau.fr/2026/02/12/jai-pris-la-decision-detre-candidat-a-lelection-presidentielle/',
    declarationSourceLabel: 'Avec Retailleau — Déclaration de candidature',
    officialSiteUrl: 'https://www.avecretailleau.fr/',
    programUrl: 'https://www.avecretailleau.fr/2026/02/12/jai-pris-la-decision-detre-candidat-a-lelection-presidentielle/',
    organization: {
      id: 'les-republicains',
      label: 'Les Républicains',
      officialUrl: 'https://republicains.fr/',
      sourceUrl: 'https://republicains.fr/actualites/2026/04/20/bruno-retailleau-largement-designe-comme-candidat-des-republicains-pour-lelection-presidentielle/',
      sourceLabel: 'Les Républicains — Désignation du candidat pour l’élection présidentielle',
    },
    proposals: [
      {
        familyCode: 'immigration',
        excerpt: 'Pour réduire drastiquement l’immigration, engager une véritable révolution de notre justice pénale, et redonner la primauté à notre droit national dès lors qu’il s’agit de protéger nos intérêts fondamentaux.',
        sourceUrl: 'https://www.avecretailleau.fr/2026/02/12/jai-pris-la-decision-detre-candidat-a-lelection-presidentielle/',
        sourceLabel: 'Avec Retailleau — Déclaration de candidature',
      },
      {
        familyCode: 'justice-securite',
        excerpt: 'Je ferai aussi respecter l’État, pour imposer partout l’autorité de la République. À nos frontières, dans nos rues.',
        sourceUrl: 'https://www.avecretailleau.fr/2026/02/12/jai-pris-la-decision-detre-candidat-a-lelection-presidentielle/',
        sourceLabel: 'Avec Retailleau — Déclaration de candidature',
      },
      {
        familyCode: 'environnement-energie',
        excerpt: 'Je réorienterai la protection de notre environnement sur une écologie de progrès.',
        sourceUrl: 'https://www.avecretailleau.fr/2026/02/12/jai-pris-la-decision-detre-candidat-a-lelection-presidentielle/',
        sourceLabel: 'Avec Retailleau — Déclaration de candidature',
      },
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Lorsque je serai élu, je vous soumettrai directement par référendum plusieurs grands textes de loi.',
        sourceUrl: 'https://www.avecretailleau.fr/2026/02/12/jai-pris-la-decision-detre-candidat-a-lelection-presidentielle/',
        sourceLabel: 'Avec Retailleau — Déclaration de candidature',
      },
    ],
  },
  {
    id: 'raphael-glucksmann',
    displayName: 'Raphaël Glucksmann',
    declaredOn: '2026-08-23',
    declarationSourceUrl: 'https://place-publique.eu/posts/2fgDdljEIy36rgxBjRyiQZ/raphael-glucksmann-sa-lettre-aux-francais',
    declarationSourceLabel: 'Place publique — Raphaël Glucksmann, sa lettre aux Français : « Je suis candidat à la Présidence de la République. »',
    officialSiteUrl: 'https://glucks2027.fr/',
    programUrl: 'https://glucks2027.fr/revolution-ecologique/',
    organization: {
      id: 'place-publique',
      label: 'Place publique',
      officialUrl: 'https://place-publique.eu/',
      sourceUrl: 'https://place-publique.eu/posts/2fgDdljEIy36rgxBjRyiQZ/raphael-glucksmann-sa-lettre-aux-francais',
      sourceLabel: 'Place publique — Raphaël Glucksmann, sa lettre aux Français (25 août 2026) (signée « Co-Président »)',
    },
    proposals: [],
  },
  {
    id: 'nicolas-dupont-aignan',
    displayName: 'Nicolas Dupont-Aignan',
    declaredOn: '2025-03-08',
    declarationSourceUrl: 'https://www.debout-la-france.fr/actualite/je-suis-candidat-pour/',
    declarationSourceLabel: 'Debout la France — « Je suis candidat pour rendre le pouvoir aux Français et rendre sa liberté à la France ! »',
    officialSiteUrl: 'https://www.dupontaignan.fr/',
    programUrl: null,
    organization: {
      id: 'debout-la-france',
      label: 'Debout la France',
      officialUrl: 'https://www.debout-la-france.fr/',
      sourceUrl: 'https://www.debout-la-france.fr/actualite/je-suis-candidat-pour/',
      sourceLabel: 'Debout la France — « Nicolas Dupont-Aignan, président de Debout la France, a officiellement annoncé, ce samedi 8 mars, sa candidature à l’élection présidentielle de 2027. »',
    },
    proposals: [],
  },
  {
    id: 'augustin-billetdoux',
    displayName: 'Augustin Billetdoux',
    declaredOn: '2025-09-25',
    declarationSourceUrl: 'https://billetdoux2027.fr/',
    declarationSourceLabel: 'billetdoux2027.fr — « Je suis candidat à l\'élection présidentielle. »',
    officialSiteUrl: 'https://billetdoux2027.fr/',
    programUrl: null,
    organization: null,
    proposals: [],
  },
  {
    id: 'delphine-batho',
    displayName: 'Delphine Batho',
    declaredOn: '2025-11-26',
    declarationSourceUrl: 'https://www.generationecologie.fr/2025/11/26/je-suis-candidate-a-lelection-presidentielle-pour-reconstruire-une-ecologie-capable-de-gouverner/',
    declarationSourceLabel: 'Génération Écologie — « Je suis candidate à l’élection présidentielle pour reconstruire une écologie capable de gouverner »',
    officialSiteUrl: 'https://www.delphinebatho.fr/',
    programUrl: null,
    organization: {
      id: 'generation-ecologie',
      label: 'Génération Écologie',
      officialUrl: 'https://www.generationecologie.fr/',
      sourceUrl: 'https://www.generationecologie.fr/publication/investiture-de-delphine-batho-a-lelection-presidentielle/',
      sourceLabel: 'Génération Écologie — « Investiture de Delphine Batho à l’élection présidentielle » (résolution de la 15e convention nationale, 6 juin 2026)',
    },
    proposals: [],
  },
  {
    id: 'nathalie-arthaud',
    displayName: 'Nathalie Arthaud',
    declaredOn: '2025-12-07',
    declarationSourceUrl: 'https://www.lutte-ouvriere.org/mensuel/article/55e-congres-lutte-ouvriere-189173.html',
    declarationSourceLabel: 'Lutte ouvrière — « Le 55e congrès de Lutte ouvrière »',
    officialSiteUrl: 'https://www.nathalie-arthaud.info',
    programUrl: 'https://www.nathalie-arthaud.info',
    organization: {
      id: 'lutte-ouvriere',
      label: 'Lutte ouvrière',
      officialUrl: 'https://www.lutte-ouvriere.org/',
      sourceUrl: 'https://www.lutte-ouvriere.org/mensuel/article/55e-congres-lutte-ouvriere-189173.html',
      sourceLabel: 'Lutte ouvrière — « Le 55e congrès de Lutte ouvrière » (Nathalie Arthaud se portera candidate au nom de Lutte ouvrière)',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Aux travailleurs de diriger, pas aux capitalistes !',
        sourceUrl: 'https://www.nathalie-arthaud.info',
        sourceLabel: 'Site de campagne de Nathalie Arthaud — « Le Medef dicte son programme, les travailleurs doivent imposer le leur ! »',
      },
    ],
  },
  {
    id: 'juan-branco',
    displayName: 'Juan Branco',
    declaredOn: '2025-12-19',
    declarationSourceUrl: 'https://juan-branco.fr/',
    declarationSourceLabel: 'juan-branco.fr (site de Juan Branco) — « Juan Branco annonce le 19 décembre 2025 son intention de se présenter à l’élection présidentielle de 2027. »',
    officialSiteUrl: 'https://juan-branco.fr/',
    programUrl: null,
    organization: {
      id: 'les-ruches',
      label: 'Les Ruches',
      officialUrl: 'https://ruches.org/',
      sourceUrl: 'https://juan-branco.fr/',
      sourceLabel: 'juan-branco.fr — « Le mouvement citoyen des Ruches porte cette candidature déclarée »',
    },
    proposals: [],
  },
  {
    id: 'francois-ruffin',
    displayName: 'François Ruffin',
    declaredOn: '2026-01-26',
    declarationSourceUrl: 'https://francoisruffin.fr/je-suis-candidat-a-la-presidence-de-la-republique-francaise/',
    declarationSourceLabel: 'francoisruffin.fr — « Je suis candidat à la présidence de la République française » (journal de 20h de TF1, 26 janvier 2026)',
    officialSiteUrl: 'https://nouspresident.fr/',
    programUrl: 'https://nouspresident.fr/notre-programme/',
    organization: {
      id: 'debout',
      label: 'Debout !',
      officialUrl: 'https://debout.fr/',
      sourceUrl: 'https://debout.fr/notre-mouvement/',
      sourceLabel: 'Debout ! — Notre mouvement, « une plateforme nationale présidée par François Ruffin »',
    },
    proposals: [
      {
        familyCode: 'travail-emploi',
        excerpt: 'Nous construirons un « statut des travailleurs essentiels » pour les femmes de ménage, AVS, caristes, agriculteurs et tous les autres qui ont tenu le pays debout pendant le Covid : avec salaires, carrières, horaires et une retraite avancée.',
        sourceUrl: 'https://nouspresident.fr/notre-programme/',
        sourceLabel: 'Nous Président ! — Notre programme, cahier 01 « Pour les travailleurs essentiels »',
      },
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Nous mettrons fin aux intérêts personnels liés aux fonctions présidentielles et de ministre. Nos successeurs n’auront plus la liberté de fixer eux-mêmes leur salaire : nous rendrons obligatoire la cohérence du salaire du Président de la République avec l’échelle des salaires.',
        sourceUrl: 'https://nouspresident.fr/separation-argent-etat/',
        sourceLabel: 'Nous Président ! — Cahier « Pour une séparation de l’argent et de l’État »',
      },
      {
        familyCode: 'agriculture-alimentation',
        excerpt: 'Le paysan ne travaillera plus à perte et le citoyen ne mangera plus à l’aveugle. Le juste prix dans les champs, la confiance dans les assiettes : en 2032, la terre nourrit ceux qui la cultivent !',
        sourceUrl: 'https://nouspresident.fr/notre-programme/',
        sourceLabel: 'Nous Président ! — Notre programme, cahier 04 « Pour une exception agriculturelle française »',
      },
      {
        familyCode: 'environnement-energie',
        excerpt: 'Notre projet consiste donc à mettre, à grande échelle, le travail au service d’une reconstruction écologique de la société.',
        sourceUrl: 'https://nouspresident.fr/travaillisme-ecologique/',
        sourceLabel: 'Nous Président ! — Cahier « Face au chantier du siècle : pour un travaillisme écologique »',
      },
    ],
  },
  {
    id: 'francois-asselineau',
    displayName: 'François Asselineau',
    declaredOn: '2026-03-21',
    declarationSourceUrl: 'https://upr.fr/militantisme/19-ans-de-lupr-francois-asselineau-lance-sa-campagne-presidentielle-2027-et-nomme-marc-parigot-directeur-de-campagne',
    declarationSourceLabel: 'UPR — « 19 ans de l\'UPR : François Asselineau lance sa campagne présidentielle 2027 » (communiqué du 22 mars 2026 sur le discours du 21 mars)',
    officialSiteUrl: 'https://upr.fr/francois-asselineau',
    programUrl: null,
    organization: {
      id: 'upr',
      label: 'Union populaire républicaine (UPR)',
      officialUrl: 'https://upr.fr/',
      sourceUrl: 'https://upr.fr/universites/discours-de-rentree-2026-de-francois-asselineau-la-liberation-nationale',
      sourceLabel: 'UPR — « François Asselineau, président de l’Union Populaire Républicaine (UPR), a officiellement lancé sa campagne pour l’élection présidentielle de 2027. »',
    },
    proposals: [],
  },
  {
    id: 'benoit-mathieu',
    displayName: 'Benoît Mathieu',
    declaredOn: '2026-03-30',
    declarationSourceUrl: 'https://benoitmathieu2027.fr/',
    declarationSourceLabel: 'Cap 500 signatures — « Je parcours la France à vélo, à la rencontre des maires, pour réunir les 500 parrainages nécessaires à ma candidature. »',
    officialSiteUrl: 'https://benoitmathieu2027.fr/',
    programUrl: 'https://benoitmathieu2027.fr/programme',
    organization: null,
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Le plan est simple : pendant que le premier ministre gouverne, j\'organiserai une assemblée citoyenne pour réécrire la Constitution, loin des partis et des politiciens.',
        sourceUrl: 'https://benoitmathieu2027.fr/programme',
        sourceLabel: 'Cap 500 signatures — Programme 2027',
      },
    ],
  },
  {
    id: 'david-lisnard',
    displayName: 'David Lisnard',
    declaredOn: '2026-03-31',
    declarationSourceUrl: 'https://lcp.fr/actualites/je-suis-candidat-david-lisnard-quitte-lr-et-relance-le-debat-sur-une-primaire-a-droite',
    declarationSourceLabel: 'LCP — « “Je suis candidat” : David Lisnard quitte LR et relance le débat sur une primaire à droite » (citation du 20 h de France 2, 31 mars 2026)',
    officialSiteUrl: 'https://www.unenouvelleenergie.fr/david-lisnard/',
    programUrl: 'https://www.unenouvelleenergie.fr/notre-programme/',
    officialDocuments: [
      { url: 'https://nouvelle-energie-media-prod.s3.fr-par.scw.cloud/media/legacy/2026/06/Discours-de-David-Lisnard-a-Bayeux-17062026.pdf', linkedFrom: 'https://www.unenouvelleenergie.fr/notre-programme/' },
    ],
    organization: {
      id: 'nouvelle-energie',
      label: 'Nouvelle Énergie',
      officialUrl: 'https://www.unenouvelleenergie.fr/',
      sourceUrl: 'https://www.unenouvelleenergie.fr/questions/david-lisnard-est-il-candidat-a-la-presidentielle-2027/',
      sourceLabel: 'Nouvelle Énergie — « Oui. David Lisnard est candidat à l’élection présidentielle de 2027, avec le mouvement Nouvelle Énergie qu’il a fondé. »',
    },
    proposals: [
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'Là où l’encadrement par les normes européennes du Pacte de stabilité européen n’a pas suffi, la France doit désormais s’astreindre à une règle institutionnelle de limitation des déficits publics. A la fois signal positif pour ses créanciers et garde-fou contre les errements politiques, cette « règle d’or » s’appliquerait aux dépenses de fonctionnement mais pas à l’investissement.',
        sourceUrl: 'https://www.unenouvelleenergie.fr/notre-programme/etre-maitre-de-notre-destin/',
        sourceLabel: 'Nouvelle Énergie — Programme : Être maître de notre destin',
      },
      {
        familyCode: 'travail-emploi',
        excerpt: 'Une mesure essentielle sera l’instauration d’un contrat de travail unique pour les nouveaux emplois, fusionnant CDI et CDD avec des indemnités de licenciement qui seraient progressives.',
        sourceUrl: 'https://www.unenouvelleenergie.fr/notre-programme/reussir-une-nouvelle-ambition-francaise/',
        sourceLabel: 'Nouvelle Énergie — Programme : Réussir une nouvelle ambition française',
      },
      {
        familyCode: 'protection-sociale-retraites',
        excerpt: 'La réforme des retraites : la priorité de l’augmentation de l’âge de la retraite avant la réforme d’ensemble combinant la répartition et la capitalisation.',
        sourceUrl: 'https://www.unenouvelleenergie.fr/notre-programme/reussir-une-nouvelle-ambition-francaise/',
        sourceLabel: 'Nouvelle Énergie — Programme : Réussir une nouvelle ambition française',
      },
      {
        familyCode: 'justice-securite',
        excerpt: 'La première priorité pour la sécurité est celle d’un plan massif de remise à niveau de la police nationale, victime du « rationnement budgétaire » au profit de l’Etat-Providence et au détriment de l’Etat régalien. Pour garantir son effectivité et son efficacité ce plan doit être déployé sur 10 ans.',
        sourceUrl: 'https://www.unenouvelleenergie.fr/notre-programme/reussir-une-nouvelle-ambition-francaise/',
        sourceLabel: 'Nouvelle Énergie — Programme : Réussir une nouvelle ambition française',
      },
    ],
  },
  {
    id: 'clara-egger',
    displayName: 'Clara Egger',
    declaredOn: '2026-04-19',
    declarationSourceUrl: 'https://solutiondemocratique.fr/presidentielle2027-annonce-candidature-clara-egger/',
    declarationSourceLabel: 'Solution démocratique (communiqué) — « Présidentielle 2027 : Clara Egger annonce sa candidature à l’élection »',
    officialSiteUrl: 'https://solutiondemocratique.fr/',
    programUrl: 'https://solutiondemocratique.fr/notre-solution/',
    organization: {
      id: 'solution-democratique',
      label: 'Solution démocratique',
      officialUrl: 'https://solutiondemocratique.fr/',
      sourceUrl: 'https://solutiondemocratique.fr/clara-egger/',
      sourceLabel: 'Solution démocratique — « Clara Egger est candidate à l’élection présidentielle de 2027 sous les couleurs de Solution Démocratique »',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Solution Démocratique œuvre à l’instauration de deux nouveaux droits politiques pour tous les Français : ceux d’initier et de voter les décisions les plus importantes, c’est-à-dire les décisions constitutionnelles.',
        sourceUrl: 'https://solutiondemocratique.fr/notre-solution/',
        sourceLabel: 'Solution démocratique — Donner aux Français le contrôle de leur système politique',
      },
      {
        familyCode: 'collectivites-outre-mer',
        excerpt: 'Nous souhaitons remettre la commune au centre de l’organisation politique française. C’est à cette condition que les Français pourront réellement peser sur les décisions qui façonnent leur quotidien.',
        sourceUrl: 'https://solutiondemocratique.fr/notre-solution/',
        sourceLabel: 'Solution démocratique — Permettre aux élus locaux de ne plus subir des décisions prises loin du terrain',
      },
    ],
  },
  {
    id: 'florian-philippot',
    displayName: 'Florian Philippot',
    declaredOn: '2026-05-09',
    declarationSourceUrl: 'https://www.sudradio.fr/politique/je-suis-candidat-florian-philippot-se-lance-pour-la-presidentielle-de-2027',
    declarationSourceLabel: 'Sud Radio — « Je suis candidat » : Florian Philippot se lance pour la présidentielle de 2027 (citation de son entretien sur France 2)',
    officialSiteUrl: 'https://les-patriotes.fr/',
    programUrl: 'https://les-patriotes.fr/projetpourlafrance/',
    organization: {
      id: 'les-patriotes',
      label: 'Les Patriotes',
      officialUrl: 'https://les-patriotes.fr/',
      sourceUrl: 'https://les-patriotes.fr/comite-executif/',
      sourceLabel: 'Les Patriotes — Comité Exécutif (« Florian PHILIPPOT » en tête de liste)',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'La France doit retrouver sa souveraineté nationale en quittant toutes les instances supranationales : UE, Euro, Schengen, CEDH, OTAN, OMS. Le peuple français pourra alors reprendre en main son destin.',
        sourceUrl: 'https://les-patriotes.fr/wp-content/uploads/2026/09/Projet-pour-la-France-2026-v2.pdf#page=2',
        sourceLabel: 'Les Patriotes — Grandes orientations pour un projet patriote, édition 2026-2027 : Démocratie, institutions, corruption',
      },
      {
        familyCode: 'environnement-energie',
        excerpt: 'Quitter le marché européen de l’électricité. Les règles du marché européen de l’électricité ont été pensées pour nuire à la France afin que celle-ci ne profite pas de l’avantage compétitif que lui procure le nucléaire. C’est ce qui fait que le prix de l’électricité est indexé sur les prix du gaz et non sur le coût réel de production.',
        sourceUrl: 'https://les-patriotes.fr/wp-content/uploads/2026/09/Projet-pour-la-France-2026-v2.pdf#page=3',
        sourceLabel: 'Les Patriotes — Grandes orientations pour un projet patriote, édition 2026-2027 : Énergie',
      },
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'Réindustrialiser l’économie française. Face à nos concurrents directs, la France doit recomposer son tissu industriel, car c’est lui qui génère le plus d’emplois induits dans les autres secteurs et qui crée la valeur ajoutée.',
        sourceUrl: 'https://les-patriotes.fr/wp-content/uploads/2026/09/Projet-pour-la-France-2026-v2.pdf#page=3',
        sourceLabel: 'Les Patriotes — Grandes orientations pour un projet patriote, édition 2026-2027 : Industrie, commerce et emploi',
      },
      {
        familyCode: 'agriculture-alimentation',
        excerpt: 'Mettre fin aux accords de libre-échange, qui ruinent nos agriculteurs en les exposant à une concurrence déloyale avec le reste du monde.',
        sourceUrl: 'https://les-patriotes.fr/wp-content/uploads/2026/09/Projet-pour-la-France-2026-v2.pdf#page=4',
        sourceLabel: 'Les Patriotes — Grandes orientations pour un projet patriote, édition 2026-2027 : Agriculture',
      },
    ],
  },
  {
    id: 'jean-francois-legendre',
    displayName: 'Jean-François Legendre',
    declaredOn: '2026-05-23',
    declarationSourceUrl: 'https://www.anousmaintenant.fr/',
    declarationSourceLabel: 'À nous maintenant — « Jean-François Legendre propose sa candidature au nom du mouvement « À nous maintenant ». »',
    officialSiteUrl: 'https://www.anousmaintenant.fr/',
    programUrl: 'https://www.anousmaintenant.fr/',
    officialDocuments: [
      { url: 'https://docs.google.com/document/d/1G4dUZaI20umZghEgSgRGCQeiWDJ2fiwhLZvtUK7hofc/edit?usp=sharing', linkedFrom: 'https://www.anousmaintenant.fr/' },
    ],
    organization: {
      id: 'a-nous-maintenant',
      label: 'À nous maintenant',
      officialUrl: 'https://www.anousmaintenant.fr/',
      sourceUrl: 'https://www.anousmaintenant.fr/',
      sourceLabel: 'À nous maintenant — « Jean-François Legendre propose sa candidature au nom du mouvement « À nous maintenant ». »',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'L\'instauration d\'un véritable dialogue structuré entre le gouvernement et les citoyens, permettant de mettre en place de nouvelles législations et s\'appuyant sur des référendums (RIC et RIG), permettra de pacifier le pays et d\'identifier les solutions aux difficultés actuelles.',
        sourceUrl: 'https://www.anousmaintenant.fr/',
        sourceLabel: 'À nous maintenant — Vers une alternative citoyenne en France — pilier 1 « Restaurer la démocratie »',
      },
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'Un redressement responsable, basé sur la justice s\'impose : voter des budgets à l\'équilibre en maîtrisant les dépenses, rationaliser les subventions, et lutter fermement contre la spéculation et l\'optimisation fiscale pour préserver les générations futures.',
        sourceUrl: 'https://www.anousmaintenant.fr/',
        sourceLabel: 'À nous maintenant — Vers une alternative citoyenne en France — pilier 2 « Sortir de l\'engrenage de la dette »',
      },
      {
        familyCode: 'collectivites-outre-mer',
        excerpt: 'Pour enrayer le déclin des campagnes et la saturation urbaine, nous encouragerons un vaste mouvement de population vers les zones rurales et les villes moyennes. Un ministère dédié sera créé en lien direct avec les maires.',
        sourceUrl: 'https://www.anousmaintenant.fr/',
        sourceLabel: 'À nous maintenant — Vers une alternative citoyenne en France — pilier 3 « Revitaliser les territoires »',
      },
      {
        familyCode: 'environnement-energie',
        excerpt: 'Face aux crises écologiques, la protection de l\'environnement ne doit pas être un sujet de division ni un empilement de contraintes punitives, mais un projet de bon sens fondé sur la préservation des ressources, du cadre de vie et du vivant.',
        sourceUrl: 'https://www.anousmaintenant.fr/',
        sourceLabel: 'À nous maintenant — Vers une alternative citoyenne en France — pilier 4 « Respecter l\'environnement »',
      },
    ],
  },
  {
    id: 'anasse-kazib',
    displayName: 'Anasse Kazib',
    declaredOn: '2026-06-01',
    declarationSourceUrl: 'https://www.revolutionpermanente.fr/Anasse-Kazib-2027-une-candidature-ouvriere-communiste-et-revolutionnaire-a-la-presidentielle',
    declarationSourceLabel: 'Révolution permanente — « Anasse Kazib 2027 : une candidature ouvrière, communiste et révolutionnaire à la présidentielle »',
    officialSiteUrl: 'https://anasse2027.fr/',
    programUrl: 'https://anasse2027.fr/programme',
    organization: {
      id: 'revolution-permanente',
      label: 'Révolution permanente',
      officialUrl: 'https://www.revolutionpermanente.fr/',
      sourceUrl: 'https://www.revolutionpermanente.fr/Anasse-Kazib-2027-une-candidature-ouvriere-communiste-et-revolutionnaire-a-la-presidentielle',
      sourceLabel: 'Révolution permanente — « Avec Révolution Permanente, Anasse Kazib est candidat à la présidentielle »',
    },
    proposals: [
      {
        familyCode: 'pouvoir-achat',
        excerpt: 'SMIC à 2000 € net, revalorisation de toutes les allocations, augmentation des salaires de 500 €, indexation sur l\'inflation.',
        sourceUrl: 'https://anasse2027.fr/programme',
        sourceLabel: 'Anasse Kazib 2027 — Programme (« Nos propositions », premières mesures)',
      },
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'Suppression des aides publiques aux grandes entreprises.',
        sourceUrl: 'https://anasse2027.fr/programme',
        sourceLabel: 'Anasse Kazib 2027 — Programme (« Nos propositions », premières mesures)',
      },
      {
        familyCode: 'travail-emploi',
        excerpt: 'Fin de la sous-traitance et CDI pour tous·tes. Même travail, même salaire.',
        sourceUrl: 'https://anasse2027.fr/programme',
        sourceLabel: 'Anasse Kazib 2027 — Programme (« Nos propositions », premières mesures)',
      },
      {
        familyCode: 'societe-libertes',
        excerpt: 'Abrogation de toutes les lois sécuritaires, racistes et islamophobes, à commencer par la loi de 2004.',
        sourceUrl: 'https://anasse2027.fr/programme',
        sourceLabel: 'Anasse Kazib 2027 — Programme (« Nos propositions », premières mesures)',
      },
    ],
  },
  {
    id: 'karim-bouamrane',
    displayName: 'Karim Bouamrane',
    declaredOn: '2026-06-09',
    declarationSourceUrl: 'https://karimbouamrane2027.org/manifeste.html',
    declarationSourceLabel: 'Karim Bouamrane 2027 — Le manifeste : « Je suis candidat à la présidence de la République »',
    officialSiteUrl: 'https://karimbouamrane2027.org/',
    programUrl: 'https://karimbouamrane2027.org/manifeste.html',
    organization: null,
    proposals: [
      {
        familyCode: 'justice-securite',
        excerpt: 'Une France humaine, c’est une France qui protège. Une France où chacun peut se sentir en sécurité, où les enfants peuvent rentrer chez eux sans traverser des points de deal, où le narcotrafic est combattu avec détermination, où l\'autorité de la République s\'exerce sur tout le territoire.',
        sourceUrl: 'https://karimbouamrane2027.org/manifeste.html',
        sourceLabel: 'Karim Bouamrane 2027 — Le manifeste de Karim Bouamrane',
      },
      {
        familyCode: 'logement',
        excerpt: 'Une France humaine, c’est une France qui loge. Une France qui permet à chacun de vivre dignement, qui construit, qui rénove, qui accompagne les plus fragiles, les familles, les jeunes actifs, les personnes âgées.',
        sourceUrl: 'https://karimbouamrane2027.org/manifeste.html',
        sourceLabel: 'Karim Bouamrane 2027 — Le manifeste de Karim Bouamrane',
      },
      {
        familyCode: 'sante',
        excerpt: 'Une France humaine, c’est une France qui soigne. Une France où chacun peut accéder facilement à des soins de qualité, quels que soient son lieu de résidence, ses revenus, son origine, sa couleur de peau, son orientation sexuelle, son genre, sa religion.',
        sourceUrl: 'https://karimbouamrane2027.org/manifeste.html',
        sourceLabel: 'Karim Bouamrane 2027 — Le manifeste de Karim Bouamrane',
      },
      {
        familyCode: 'education-culture',
        excerpt: 'Je veux que chaque jeune puisse accéder à une formation qui mène à un emploi choisi et non subi.',
        sourceUrl: 'https://karimbouamrane2027.org/manifeste.html',
        sourceLabel: 'Karim Bouamrane 2027 — Le manifeste de Karim Bouamrane',
      },
    ],
  },
  {
    id: 'selma-labib',
    displayName: 'Selma Labib',
    declaredOn: '2026-06-17',
    declarationSourceUrl: 'https://npa-revolutionnaires.org/articles/communique-presidentielle-2027-selma-labib-une-candidature-ouvriere-et-revolutionnaire-soutenue-par-gael-quirante',
    declarationSourceLabel: 'NPA-Révolutionnaires — « Communiqué – Présidentielle 2027 : Selma Labib, une candidature ouvrière et révolutionnaire, soutenue par Gaël Quirante »',
    officialSiteUrl: 'https://npa-revolutionnaires.org/campagnes-presidentielles',
    programUrl: 'https://npa-revolutionnaires.org/articles/cest-au-x-travailleurs-et-aux-travailleuses-de-diriger-la-societe',
    organization: {
      id: 'npa-revolutionnaires',
      label: 'NPA-Révolutionnaires',
      officialUrl: 'https://npa-revolutionnaires.org/',
      sourceUrl: 'https://npa-revolutionnaires.org/articles/communique-presidentielle-2027-selma-labib-une-candidature-ouvriere-et-revolutionnaire-soutenue-par-gael-quirante',
      sourceLabel: 'NPA-Révolutionnaires — communiqué du 17 juin 2026 (le comité politique national a décidé de présenter Selma Labib ; elle fera campagne au nom du NPA-Révolutionnaires)',
    },
    proposals: [
      {
        familyCode: 'international-defense',
        excerpt: 'À BAS LE MILITARISME, PAS UNE FEMME, PAS UN HOMME, PAS UN EURO POUR LES GUERRES IMPÉRIALISTES',
        sourceUrl: 'https://npa-revolutionnaires.org/wp-content/uploads/2026/09/260919_4p-campagne-A4.pdf#page=2',
        sourceLabel: 'NPA-Révolutionnaires — « C’est aux travailleurs et aux travailleuses de diriger la société ! » (tract 4 pages de campagne, sept. 2026)',
      },
      {
        familyCode: 'pouvoir-achat',
        excerpt: 'L’augmentation massive, générale des salaires mais aussi des pensions et des allocations, et leur indexation sur les prix : c’est le premier objectif que doivent se fixer les travailleurs, c’est une urgence pour l’ensemble du monde du travail !',
        sourceUrl: 'https://npa-revolutionnaires.org/wp-content/uploads/2026/09/260919_4p-campagne-A4.pdf#page=2',
        sourceLabel: 'NPA-Révolutionnaires — « C’est aux travailleurs et aux travailleuses de diriger la société ! » (tract 4 pages de campagne, sept. 2026)',
      },
      {
        familyCode: 'travail-emploi',
        excerpt: 'Imposer l’interdiction des licenciements, l’arrêt de toutes les suppressions d’emplois et la baisse du temps de travail sans perte de salaire, jusqu’à l’extinction du chômage, c’est la seule manière de mettre fin au chômage de masse, pour les jeunes comme pour les anciens.',
        sourceUrl: 'https://npa-revolutionnaires.org/wp-content/uploads/2026/09/260919_4p-campagne-A4.pdf#page=2',
        sourceLabel: 'NPA-Révolutionnaires — « C’est aux travailleurs et aux travailleuses de diriger la société ! » (tract 4 pages de campagne, sept. 2026)',
      },
    ],
  },
  {
    id: 'antoine-mikolajczak',
    displayName: 'Antoine Mikolajczak',
    declaredOn: '2026-06-27',
    declarationSourceUrl: 'https://parti-equinoxe.fr/retour-sur-le-lancement-de-campagne-dequinoxe/',
    declarationSourceLabel: 'Équinoxe — « Retour sur le lancement de campagne d’Équinoxe » : « notre campagne présidentielle lancée le samedi 27 juin dernier à Saint-Père-Sur-Loire, dans le Loiret. Portée par notre candidat, Antoine Mikolajczak »',
    officialSiteUrl: 'https://parti-equinoxe.fr/programme-2027/',
    programUrl: 'https://parti-equinoxe.fr/programme-2027/',
    organization: {
      id: 'equinoxe',
      label: 'Équinoxe',
      officialUrl: 'https://parti-equinoxe.fr/',
      sourceUrl: 'https://parti-equinoxe.fr/programme-2027/',
      sourceLabel: 'Équinoxe — Programme 2027 : « Découvrez Antoine Mikolajczak, candidat Équinoxe à l’élection présidentielle de 2027. »',
    },
    proposals: [
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Rénover la démocratie pour que les citoyens et citoyennes en soient vraiment les acteurs, et simplifier les règles pour que chacun puisse les comprendre et les respecter.',
        sourceUrl: 'https://parti-equinoxe.fr/programme-2027/',
        sourceLabel: 'Équinoxe — Programme 2027 — priorité « Décider collectivement »',
      },
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'Relocaliser les productions agricoles et industrielles, pour reprendre la main et choisir ce que l’on produit, dans le respect des limites planétaires.',
        sourceUrl: 'https://parti-equinoxe.fr/programme-2027/',
        sourceLabel: 'Équinoxe — Programme 2027 — priorité « Produire en France »',
      },
      {
        familyCode: 'sante',
        excerpt: 'Garantir à toutes et tous un accès à des soins adaptés : installations de médecins liées aux besoins prioritaires, consultations flexibles dans les zones sous-dotées, temps médical libéré.',
        sourceUrl: 'https://parti-equinoxe.fr/programme-2027/',
        sourceLabel: 'Équinoxe — Programme 2027 — priorité « Vivre en bonne santé »',
      },
    ],
  },
  {
    id: 'francis-lalanne',
    displayName: 'Francis Lalanne',
    declaredOn: '2026-08-16',
    declarationSourceUrl: 'https://dieudolalanne2027.com/conference-de-presse-au-theatre-galabru-le-22-aout-20h/',
    declarationSourceLabel: 'Dieudo Lalanne 2027 (site officiel de campagne) — « Francis Lalanne, candidat aux élections présidentielles 2027, informe la presse qu’il fera un meeting au Théâtre Galabru »',
    officialSiteUrl: 'https://dieudolalanne2027.com/',
    programUrl: 'https://dieudolalanne2027.com/programme',
    organization: {
      id: 'france-libre',
      label: 'France Libre',
      officialUrl: 'https://france-libre.fr/',
      sourceUrl: 'https://france-libre.fr/a-propos/',
      sourceLabel: 'France Libre — « À Propos de Nous : La Résistance » (Président : Francis LALANNE)',
    },
    proposals: [
      {
        familyCode: 'international-defense',
        excerpt: 'Frexit organisé : Sortie de l\'Union Européenne pour retrouver la maîtrise de nos lois et de nos frontières.',
        sourceUrl: 'https://dieudolalanne2027.com/programme',
        sourceLabel: 'Dieudo Lalanne 2027 (site de campagne) — Le Projet 2027 (1. Souveraineté !)',
      },
      {
        familyCode: 'societe-libertes',
        excerpt: 'Liberté d\'expression totale : Abrogation des lois liberticides qui musèlent le débat public.',
        sourceUrl: 'https://dieudolalanne2027.com/programme',
        sourceLabel: 'Dieudo Lalanne 2027 (site de campagne) — Le Projet 2027 (3. Liberté !)',
      },
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'RIC (Référendum d\'Initiative Citoyenne) : Inscription du RIC en toutes matières dans la Constitution. Création d’une assemblée constituante.',
        sourceUrl: 'https://dieudolalanne2027.com/programme',
        sourceLabel: 'Dieudo Lalanne 2027 (site de campagne) — Le Projet 2027 (4. Démocratie !)',
      },
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'Baisse radicale des impôts et surtout des taxes sur les carburants.',
        sourceUrl: 'https://dieudolalanne2027.com/programme',
        sourceLabel: 'Dieudo Lalanne 2027 (site de campagne) — Le Projet 2027 (5. Stop Impôts !)',
      },
    ],
  },
  {
    id: 'erwan-rohou',
    displayName: 'Erwan Rohou',
    declaredOn: '2026-09-04',
    declarationSourceUrl: 'https://www.les-ecologistes-mei.org/post/erwan-rohou-candidat-du-mei-aux-%C3%A9lections-pr%C3%A9sidentielles',
    declarationSourceLabel: 'Mouvement écologiste indépendant — « Erwan ROHOU, candidat du MEI aux élections présidentielles. » (« Ce candidat sera Erwan Rohou. »)',
    officialSiteUrl: 'https://www.les-ecologistes-mei.org/',
    programUrl: 'https://www.les-ecologistes-mei.org/post/le-livre-blanc-du-mouvement-ecologiste-ind%C3%A9pendant-1',
    organization: {
      id: 'mouvement-ecologiste-independant',
      label: 'Mouvement écologiste indépendant',
      officialUrl: 'https://www.les-ecologistes-mei.org/',
      sourceUrl: 'https://www.les-ecologistes-mei.org/post/erwan-rohou-candidat-du-mei-aux-%C3%A9lections-pr%C3%A9sidentielles',
      sourceLabel: 'MEI — « Erwan ROHOU, candidat du MEI aux élections présidentielles. » (membre du bureau national du MEI)',
    },
    proposals: [
      {
        familyCode: 'justice-securite',
        excerpt: 'Le Livre Blanc du Mouvement Ecologiste Indépendant propose une réforme juridique structurante : créer un Pacte juridique pour la Nature, fondé sur cinq constats et trois niveaux de transformation.',
        sourceUrl: 'https://www.les-ecologistes-mei.org/post/le-livre-blanc-du-mouvement-ecologiste-ind%C3%A9pendant-1',
        sourceLabel: 'MEI — Livre Blanc pour la Protection de l’environnement, Éléments de Programmatique Présidentielle 2027 (Le Pacte juridique pour la Nature)',
      },
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Le Livre Blanc du Mouvement Ecologiste Indépendant propose de réarmer l’État pour qu’il redevienne un garant impartial de la protection du vivant.',
        sourceUrl: 'https://www.les-ecologistes-mei.org/post/le-livre-blanc-du-mouvement-ecologiste-ind%C3%A9pendant-1',
        sourceLabel: 'MEI — Livre Blanc pour la Protection de l’environnement, Éléments de Programmatique Présidentielle 2027 (Restaurer la confiance : une gouvernance indépendante et transparente)',
      },
      {
        familyCode: 'environnement-energie',
        excerpt: 'Le Livre Blanc du Mouvement Ecologiste Indépendant propose un programme national de restauration écologique, structuré autour de dix axes opérationnels.',
        sourceUrl: 'https://www.les-ecologistes-mei.org/post/le-livre-blanc-du-mouvement-ecologiste-ind%C3%A9pendant-1',
        sourceLabel: 'MEI — Livre Blanc pour la Protection de l’environnement, Éléments de Programmatique Présidentielle 2027 (Réparer la Nature : un programme national de restauration écologique)',
      },
    ],
  },
  {
    id: 'fabien-roussel',
    displayName: 'Fabien Roussel',
    declaredOn: '2026-09-06',
    declarationSourceUrl: 'https://www.pcf.fr/presidentielle2027_fabien_roussel_designe_candidat',
    declarationSourceLabel: 'PCF — « Élection présidentielle : Fabien Roussel désigné candidat à 72% par les communistes »',
    officialSiteUrl: 'https://www.fabienroussel2027.fr/',
    programUrl: null,
    officialDocuments: [
      { url: 'https://assets.nationbuilder.com/pcf/pages/44585/attachments/original/1791210591/tract_service_public.pdf', linkedFrom: 'https://www.fabienroussel2027.fr/materiel' },
      { url: 'https://assets.nationbuilder.com/pcf/pages/44583/attachments/original/1791210341/tract_pompier.pdf', linkedFrom: 'https://www.fabienroussel2027.fr/materiel' },
      { url: 'https://assets.nationbuilder.com/pcf/pages/44609/attachments/original/1791282014/Plan_Jeunesse_2027-2032.pdf', linkedFrom: 'https://www.fabienroussel2027.fr/materiel' },
    ],
    organization: {
      id: 'parti-communiste-francais',
      label: 'Parti communiste français',
      officialUrl: 'https://www.pcf.fr/',
      sourceUrl: 'https://www.pcf.fr/presidentielle2027_fabien_roussel_designe_candidat',
      sourceLabel: 'PCF — « Élection présidentielle : Fabien Roussel désigné candidat à 72% par les communistes »',
    },
    proposals: [],
  },
  {
    id: 'eric-zemmour',
    displayName: 'Éric Zemmour',
    declaredOn: '2026-09-17',
    declarationSourceUrl: 'https://lcp.fr/actualites/presidentielle-2027-eric-zemmour-officialise-sa-candidature-441871',
    declarationSourceLabel: 'LCP — « Présidentielle 2027: Éric Zemmour officialise sa candidature » (entretien BFMTV du 17 septembre 2026)',
    officialSiteUrl: 'https://www.ericzemmour.fr/',
    programUrl: null,
    organization: {
      id: 'reconquete',
      label: 'Reconquête',
      officialUrl: 'https://www.parti-reconquete.fr/',
      sourceUrl: 'https://www.parti-reconquete.fr/',
      sourceLabel: 'Reconquête — Accueil, « L’action de reconquête »',
    },
    proposals: [],
  },
  {
    id: 'olivier-becht',
    displayName: 'Olivier Becht',
    declaredOn: '2026-09-24',
    declarationSourceUrl: 'https://www.sudradio.fr/politique/olivier-becht-je-suis-candidat-a-lelection-presidentielle-et-jirai-jusquau-bout',
    declarationSourceLabel: 'Sud Radio (entretien du 11 octobre 2026) — « Je suis candidat à l’élection présidentielle et j’irai jusqu’au bout ! »',
    officialSiteUrl: 'https://becht2027.fr/',
    programUrl: 'https://becht2027.fr/projet-france/les-5-refondations/',
    organization: {
      id: 'le-projet-france',
      label: 'Le Projet France',
      officialUrl: 'https://becht2027.fr/',
      sourceUrl: 'https://becht2027.fr/mentions-legales/',
      sourceLabel: 'becht2027.fr — Mentions légales : site édité par « le Projet France, association loi de 1901 », directeur de la publication Olivier Becht',
    },
    proposals: [
      {
        familyCode: 'environnement-energie',
        excerpt: 'Nous allons développer une énergie nouvelle, les carburants de synthèse, entièrement décarbonée qui rendra la France totalement indépendante et offrira aux ménages comme aux entreprises des prix bas et stables.',
        sourceUrl: 'https://becht2027.fr/projet-france/les-5-refondations/',
        sourceLabel: 'becht2027.fr — Les 5 refondations — L’énergie',
      },
      {
        familyCode: 'protection-sociale-retraites',
        excerpt: 'Nous allons sortir du modèle de l’assurance sociale pour aller vers un « contrat universel » qui protègera mieux tous les Français, de la naissance à la mort, contre tous les risques.',
        sourceUrl: 'https://becht2027.fr/projet-france/les-5-refondations/',
        sourceLabel: 'becht2027.fr — Les 5 refondations — La refondation sociale',
      },
      {
        familyCode: 'sante',
        excerpt: 'Nous allons refonder totalement notre système de santé à la fois par une alimentation saine sans phytosanitaires, par une nouvelle organisation des soins et par un défi technologique qui permettra de vaincre le cancer, Alzheimer, ou Parkinson.',
        sourceUrl: 'https://becht2027.fr/projet-france/les-5-refondations/',
        sourceLabel: 'becht2027.fr — Les 5 refondations — La santé',
      },
      {
        familyCode: 'justice-securite',
        excerpt: 'Nous mettrons en place un grand plan de sécurité en reprenant à la base l’éducation, en gardant très longtemps en prison ceux qui menacent la société et en créant des centres de redressement encadrés par des anciens militaires où les délinquants travailleront pour réparer leurs fautes.',
        sourceUrl: 'https://becht2027.fr/projet-france/les-5-refondations/',
        sourceLabel: 'becht2027.fr — Les 5 refondations — La sécurité',
      },
    ],
  },
  {
    id: 'mira-markovic',
    displayName: 'Mira Markovic',
    declaredOn: '2026-10-02',
    declarationSourceUrl: 'https://parti-animaliste.fr/news/communique-de-presse/mira-markovic-representera-le-parti-animaliste-a-la-presidentielle',
    declarationSourceLabel: 'Parti animaliste (communiqué de presse du 2 octobre 2026) — « Mira Markovic représentera le Parti animaliste à la présidentielle »',
    officialSiteUrl: 'https://parti-animaliste.fr/mira-markovic-2027',
    programUrl: null,
    organization: {
      id: 'parti-animaliste',
      label: 'Parti animaliste',
      officialUrl: 'https://parti-animaliste.fr/',
      sourceUrl: 'https://parti-animaliste.fr/news/communique-de-presse/mira-markovic-representera-le-parti-animaliste-a-la-presidentielle',
      sourceLabel: 'Parti animaliste — « Mira Markovic représentera le Parti animaliste à la présidentielle » (cofondatrice, membre du Bureau national)',
    },
    proposals: [],
  },
  {
    id: 'gilles-platret',
    displayName: 'Gilles Platret',
    declaredOn: '2026-10-08',
    declarationSourceUrl: 'https://www.ici.fr/emissions/l-invite-d-ici-matin-ici-bourgogne/modeste-mais-determine-le-maire-de-chalon-sur-saone-gilles-platret-se-declare-candidat-a-la-presidentielle-6724488',
    declarationSourceLabel: 'ICI Bourgogne (entretien du 9 octobre 2026, au lendemain de sa réunion publique « Parlons de la France ») — « Êtes-vous candidat ? » « Oui »',
    officialSiteUrl: null,
    programUrl: null,
    organization: null,
    proposals: [],
  },
  {
    id: 'natacha-polony',
    displayName: 'Natacha Polony',
    declaredOn: '2026-10-14',
    declarationSourceUrl: 'https://www.npolony.fr/videos?v=tQ45aZtNEHk',
    declarationSourceLabel: 'npolony.fr (site de campagne) — « Natacha Polony - Ma déclaration de candidature à l’élection présidentielle de 2027 »',
    officialSiteUrl: 'https://natachapolony2027.fr/',
    programUrl: 'https://natachapolony2027.fr/natacha-polony/ma-vision',
    organization: {
      id: 'nous-sommes-la-france',
      label: 'Nous sommes la France',
      officialUrl: 'https://natachapolony2027.fr/',
      sourceUrl: 'https://natachapolony2027.fr/',
      sourceLabel: 'natachapolony2027.fr — « Natacha Polony 2027 | Nous sommes la France »',
    },
    proposals: [
      {
        familyCode: 'education-culture',
        excerpt: 'L’école doit donner à chaque enfant les connaissances et la maîtrise de la langue qui permettent de comprendre le monde et de former son jugement.',
        sourceUrl: 'https://natachapolony2027.fr/',
        sourceLabel: 'natachapolony2027.fr — « Retrouvons les moyens d’agir » — Transmettre pour émanciper',
      },
      {
        familyCode: 'economie-finances-publiques',
        excerpt: 'Nos choix collectifs dépendent aussi de ce que nous savons fabriquer, cultiver et préserver. Le travail, la production et l’écologie doivent être pensés ensemble.',
        sourceUrl: 'https://natachapolony2027.fr/',
        sourceLabel: 'natachapolony2027.fr — « Retrouvons les moyens d’agir » — Produire pour choisir',
      },
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'La démocratie prend tout son sens lorsque les citoyens peuvent peser sur les décisions qui engagent leur avenir.',
        sourceUrl: 'https://natachapolony2027.fr/',
        sourceLabel: 'natachapolony2027.fr — « Retrouvons les moyens d’agir » — Faire vivre la démocratie',
      },
    ],
  },
  {
    id: 'teddy-riner',
    displayName: 'Teddy Riner',
    declaredOn: '2026-11-09',
    declarationSourceUrl: 'https://www.leparisien.fr/elections/presidentielle/presidentielle-2027-je-suis-candidat-pour-dire-quon-peut-relever-la-france-teddy-riner-se-lance-dans-la-competition-09-11-2026-36MFSFMWDFGATLM2YGR3MDF4QI.php',
    declarationSourceLabel: 'Le Parisien (entretien du 9 novembre 2026) — « Je suis candidat pour dire qu’on peut relever la France »',
    officialSiteUrl: null,
    programUrl: null,
    organization: null,
    proposals: [],
  },
]

// Extraits relevés programme par programme. Présence mot pour mot : scripts/check-candidate-sources.mjs.
const extractedProposals = JSON.parse(readFileSync(new URL('./candidates-2027-proposals.json', import.meta.url), 'utf8'))
const candidatesById = new Map(candidates.map((candidate) => [candidate.id, candidate]))
for (const { candidateId, ...proposal } of extractedProposals) {
  const candidate = candidatesById.get(candidateId)
  if (!candidate) throw new Error(`Extrait rattaché à une candidature inconnue : ${candidateId}`)
  candidate.proposals.push(proposal)
}

const SOCIAL_NETWORK = /(^|\.)(youtube\.com|youtu\.be|facebook\.com|instagram\.com|x\.com|twitter\.com|tiktok\.com|linkedin\.com|bsky\.app|linktr\.ee)$/
const hostOf = (url) => new URL(url).hostname.replace(/^www\./, '')

function isOnOfficialSite(candidate, url) {
  if (!candidate.officialSiteUrl) return false
  const site = hostOf(candidate.officialSiteUrl)
  const host = hostOf(url)
  return host === site || host.endsWith(`.${site}`)
}

// Une ancre « #page=N » désigne le même document.
const isLinkedDocument = (candidate, url) => (candidate.officialDocuments ?? []).some((document) => document.url === url.split('#')[0])

// Une source hors du site officiel fait échouer le script, y compris en --dry-run.
const violations = candidates.flatMap((candidate) => {
  const found = []
  const check = (label, url) => {
    if (!isOnOfficialSite(candidate, url) && !isLinkedDocument(candidate, url)) found.push(`${candidate.id} : ${label} hors du site officiel : ${url}`)
  }
  if (candidate.officialSiteUrl && SOCIAL_NETWORK.test(hostOf(candidate.officialSiteUrl))) {
    found.push(`${candidate.id} : un réseau social n'est pas un site officiel : ${candidate.officialSiteUrl}`)
  }
  if (candidate.programUrl) check('programUrl', candidate.programUrl)
  for (const proposal of candidate.proposals) check('extrait', proposal.sourceUrl)
  for (const document of candidate.officialDocuments ?? []) {
    if (!isOnOfficialSite(candidate, document.linkedFrom)) found.push(`${candidate.id} : document lié depuis une page hors du site officiel : ${document.linkedFrom}`)
  }
  return found
})
if (violations.length > 0) throw new Error(`Sources refusées (README.md §8.2) :\n${violations.join('\n')}`)

// Attendre l'écriture : process.exit() tronquerait la sortie sur un pipe.
const writeAndExit = async (text) => {
  await new Promise((resolve) => process.stdout.write(text, resolve))
  process.exit(0)
}

if (options.has('--json')) await writeAndExit(`${JSON.stringify(candidates)}\n`)

const sqlText = (value) => {
  if (value == null) return 'NULL'
  const base64 = Buffer.from(String(value)).toString('base64')
  return `convert_from(decode('${base64}', 'base64'), 'UTF8')`
}

const sqlDate = (value) => (value == null ? 'NULL' : `${sqlText(value)}::date`)
const values = (row) => row.join(', ')

function insertCandidate(candidate) {
  return `INSERT INTO presidential_candidates (
    id, display_name, declared_on, declaration_source_url, declaration_source_label, official_site_url, program_url
  ) VALUES (${values([
    sqlText(candidate.id),
    sqlText(candidate.displayName),
    sqlDate(candidate.declaredOn),
    sqlText(candidate.declarationSourceUrl),
    sqlText(candidate.declarationSourceLabel),
    sqlText(candidate.officialSiteUrl),
    sqlText(candidate.programUrl),
  ])})
  ON CONFLICT (id) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    declared_on = EXCLUDED.declared_on,
    declaration_source_url = EXCLUDED.declaration_source_url,
    declaration_source_label = EXCLUDED.declaration_source_label,
    official_site_url = EXCLUDED.official_site_url,
    program_url = EXCLUDED.program_url,
    updated_at = NOW();`
}

function insertOrganization(candidate) {
  const organization = candidate.organization
  if (!organization) return null
  return [
    `INSERT INTO political_organizations (id, label, official_url)
     VALUES (${values([sqlText(organization.id), sqlText(organization.label), sqlText(organization.officialUrl)])})
     ON CONFLICT (id) DO UPDATE SET label = EXCLUDED.label, official_url = EXCLUDED.official_url;`,
    `INSERT INTO candidate_political_organizations (candidate_id, organization_id, source_url, source_label)
     VALUES (${values([sqlText(candidate.id), sqlText(organization.id), sqlText(organization.sourceUrl), sqlText(organization.sourceLabel)])})
     ON CONFLICT (candidate_id, organization_id) DO UPDATE SET
       source_url = EXCLUDED.source_url,
       source_label = EXCLUDED.source_label;`,
  ].join('\n')
}

function insertProposal(candidate, proposal) {
  return `INSERT INTO candidate_program_proposals (
    candidate_id, family_code, excerpt, source_url, source_label, source_published_on
  ) VALUES (${values([
    sqlText(candidate.id),
    sqlText(proposal.familyCode),
    sqlText(proposal.excerpt),
    sqlText(proposal.sourceUrl),
    sqlText(proposal.sourceLabel),
    sqlDate(proposal.sourcePublishedOn),
  ])})
  ON CONFLICT (candidate_id, family_code, excerpt, source_url) DO UPDATE SET
    source_label = EXCLUDED.source_label,
    source_published_on = EXCLUDED.source_published_on;`
}

const sql = [
  'BEGIN;',
  ...candidates.map(insertCandidate),
  ...candidates.map(insertOrganization).filter(Boolean),
  // Le script fait référence : un extrait corrigé ou retiré ne doit pas survivre en base.
  `DELETE FROM candidate_program_proposals WHERE candidate_id IN (${values(candidates.map((candidate) => sqlText(candidate.id)))});`,
  ...candidates.flatMap((candidate) => candidate.proposals.map((proposal) => insertProposal(candidate, proposal))),
  'COMMIT;',
].join('\n\n')

if (dryRun) await writeAndExit(`${sql}\n`)

if (!useDocker && !process.env.DATABASE_URL) {
  throw new Error('DATABASE_URL est requis ; chargez .env avant d’exécuter le script.')
}

function run(command, args, input) {
  const result = spawnSync(command, args, { input, encoding: 'utf8', env: process.env })
  if (result.error) {
    throw new Error(`Impossible d’exécuter ${command} : ${result.error.message}`)
  }
  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || `${command} a échoué`)
  }
}

// `docker compose exec -T` tronque une entrée standard de cette taille : le SQL
// passe par un fichier copié dans le conteneur.
function runInDevContainer(script) {
  const compose = ['compose', '-f', 'compose.dev.yml']
  const localDir = mkdtempSync(join(tmpdir(), 'seed-candidates-'))
  const localFile = join(localDir, 'seed.sql')
  const containerFile = '/tmp/seed-candidates-2027.sql'
  writeFileSync(localFile, script)
  try {
    run('docker', [...compose, 'cp', localFile, `postgres:${containerFile}`])
    run('docker', [...compose, 'exec', '-T', 'postgres', 'psql', '-X', '-v', 'ON_ERROR_STOP=1', '-U', 'hemicycle', '-d', 'hemicycle_dev', '-f', containerFile])
  } finally {
    rmSync(localDir, { recursive: true, force: true })
    spawnSync('docker', [...compose, 'exec', '-T', 'postgres', 'rm', '-f', containerFile])
  }
}

if (useDocker) {
  runInDevContainer(sql)
} else {
  run('psql', ['-X', '-v', 'ON_ERROR_STOP=1', process.env.DATABASE_URL], sql)
}

process.stdout.write(`Registre 2027 alimenté : ${candidates.length} candidatures et ${candidates.reduce((count, candidate) => count + candidate.proposals.length, 0)} extraits sourcés.\n`)
