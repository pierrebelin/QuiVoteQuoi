#!/usr/bin/env node

/**
 * Peuple le registre des candidatures presidentielles 2027 dont la
 * declaration est documentee par une source primaire et dont le programme est
 * ajoute seulement lorsqu'une source primaire le permet.
 *
 * Les extraits sont reproduits tels qu'ils apparaissent dans les programmes.
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
 */
import { spawnSync } from 'node:child_process'

const options = new Set(process.argv.slice(2))
if (![...options].every((option) => option === '--dry-run' || option === '--docker')) {
  throw new Error('Usage: node scripts/seed-candidates-2027.mjs [--dry-run] [--docker]')
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
        excerpt: 'Inscrire dans la Constitution une règle d’or de maîtrise des déficits, et ramener le déficit de plus de 5 % à 2 % du PIB en fin de quinquennat, comme l’ont fait l’Allemagne ou les Pays-Bas.',
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
        familyCode: 'environnement-energie',
        excerpt: 'Assurer notre souveraineté énergétique en relançant le nucléaire, en développant les renouvelables et en électrifiant massivement nos usages.',
        sourceUrl: 'https://www.edouardphilippe.fr/priorites/pour-une-france-plus-conquerante',
        sourceLabel: 'Avec Édouard 2027 — Pour une France plus conquérante',
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
    programUrl: 'https://melenchon2027.fr/programme2025/livre/',
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
        sourceUrl: 'https://melenchon2027.fr/programme2025/livre/chapitre1/s1/',
        sourceLabel: 'L’Avenir en commun, édition 2025 — Réunir une Assemblée constituante pour passer à la 6e République',
      },
      {
        familyCode: 'pouvoir-achat',
        excerpt: 'Rendre l’impôt sur le revenu plus progressif avec un barème à 14 tranches contre 5 aujourd’hui.',
        sourceUrl: 'https://melenchon2027.fr/programme2025/livre/chapitre6/s5/',
        sourceLabel: 'L’Avenir en commun, édition 2025 — Faire la révolution fiscale',
      },
      {
        familyCode: 'environnement-energie',
        excerpt: 'Inscrire dans la Constitution le principe de la « règle verte », selon laquelle on ne prélève pas davantage à la nature que ce qu’elle est en état de reconstituer.',
        sourceUrl: 'https://melenchon2027.fr/programme2025/livre/chapitre12/s1/',
        sourceLabel: 'L’Avenir en commun, édition 2025 — La bifurcation écologique pour une société de l’harmonie',
      },
      {
        familyCode: 'international-defense',
        excerpt: 'Se retirer immédiatement du commandement intégré de l’OTAN puis, par étapes, de l’organisation elle-même.',
        sourceUrl: 'https://melenchon2027.fr/programme2025/livre/chapitre16/s1/',
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
        familyCode: 'pouvoir-achat',
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
    officialSiteUrl: null,
    programUrl: null,
    organization: null,
    proposals: [],
  },
  {
    id: 'marine-tondelier',
    displayName: 'Marine Tondelier',
    declaredOn: '2025-12-08',
    declarationSourceUrl: 'https://lesecologistes.fr/posts/4DGpyusxBAU4xQPexfncsx/marine-tondelier-designee-pour-representer-les-ecologistes-a-l-election-presidentielle',
    declarationSourceLabel: 'Les Écologistes — Désignation pour l’élection présidentielle',
    officialSiteUrl: 'https://marinetondelier.fr/',
    programUrl: 'https://lesecologistes.fr/share/page/6ImK65GKUnvibm33WGkjkj/projet',
    organization: {
      id: 'les-ecologistes',
      label: 'Les Écologistes',
      officialUrl: 'https://lesecologistes.fr/',
      sourceUrl: 'https://lesecologistes.fr/posts/4DGpyusxBAU4xQPexfncsx/marine-tondelier-designee-pour-representer-les-ecologistes-a-l-election-presidentielle',
      sourceLabel: 'Les Écologistes — Désignation pour l’élection présidentielle',
    },
    proposals: [
      {
        familyCode: 'environnement-energie',
        excerpt: 'Marine Tondelier et Les Écologistes proposent la création d’un congé climatique jusqu’à 5 jours, pour permettre à chacun·e de faire face à une canicule, une inondation, un incendie ou une fermeture d’école liée au climat, sans perte de revenus.',
        sourceUrl: 'https://action.lesecologistes.fr/petition/50hAndmVdkRyA7bvNqn2xx/conge-climatique',
        sourceLabel: 'Les Écologistes — Congé climatique',
      },
      {
        familyCode: 'institutions-vie-politique',
        excerpt: 'Nous ferons de la lutte contre la corruption une priorité, garantirons l’indépendance de la justice et des médias, et moderniserons nos institutions pour fonder la première république écologique et citoyenne, plus proche de la souveraineté populaire.',
        sourceUrl: 'https://yvelines.lesecologistes.fr/posts/6llZ3TixHP7gIKPL7zbPcx/programme-ecologiste-2027-le-futur-est-deja-la-que-serait-une-societe-ecologiste',
        sourceLabel: 'Les Écologistes — Programme écologiste 2027',
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
    declarationSourceUrl: 'https://place-publique.eu/?p=8295',
    declarationSourceLabel: 'Place publique — Raphaël Glucksmann est candidat à l’élection présidentielle',
    officialSiteUrl: 'https://glucks2027.fr/',
    programUrl: null,
    organization: {
      id: 'place-publique',
      label: 'Place publique',
      officialUrl: 'https://place-publique.eu/',
      sourceUrl: 'https://place-publique.eu/?p=8295',
      sourceLabel: 'Place publique — Raphaël Glucksmann est candidat à l’élection présidentielle',
    },
    proposals: [],
  },
]

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
  ...candidates.flatMap((candidate) => candidate.proposals.map((proposal) => insertProposal(candidate, proposal))),
  'COMMIT;',
].join('\n\n')

if (dryRun) {
  process.stdout.write(`${sql}\n`)
  process.exit(0)
}

if (!useDocker && !process.env.DATABASE_URL) {
  throw new Error('DATABASE_URL est requis ; chargez .env avant d’exécuter le script.')
}

const command = useDocker ? 'docker' : 'psql'
const args = useDocker
  ? ['compose', '-f', 'compose.dev.yml', 'exec', '-T', 'postgres', 'psql', '-X', '-v', 'ON_ERROR_STOP=1', '-U', 'hemicycle', '-d', 'hemicycle_dev']
  : ['-X', '-v', 'ON_ERROR_STOP=1', process.env.DATABASE_URL]
const result = spawnSync(command, args, {
  input: sql,
  encoding: 'utf8',
  env: process.env,
})

if (result.error) {
  throw new Error(`Impossible d’exécuter ${command} : ${result.error.message}`)
}
if (result.status !== 0) {
  throw new Error(result.stderr || result.stdout || 'psql a échoué')
}

process.stdout.write(`Registre 2027 alimenté : ${candidates.length} candidatures et ${candidates.reduce((count, candidate) => count + candidate.proposals.length, 0)} extraits sourcés.\n`)
