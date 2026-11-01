#!/usr/bin/env node

/**
 * Copie dans le PostgreSQL Docker les faits publics nécessaires à une fiche
 * dossier locale. L'API de production est la source ; aucune écriture ne lui
 * est adressée. Usage :
 *
 *   QUIVOTEQUOI_PUBLIC_BASE_URL=https://<DOMAINE_PUBLIC> \
 *     node scripts/import-public-dossier.mjs DLR5L17N51732
 */
import { spawnSync } from 'node:child_process'

const uid = process.argv[2]
if (!uid) throw new Error('Usage: node scripts/import-public-dossier.mjs <UID_DOSSIER>')

const publicBaseUrl = process.env.QUIVOTEQUOI_PUBLIC_BASE_URL
if (!publicBaseUrl) throw new Error('QUIVOTEQUOI_PUBLIC_BASE_URL est requis')

const base = new URL('/api', publicBaseUrl).toString()
const fetchJson = async (path) => {
  const response = await fetch(`${base}${path}`)
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`)
  return response.json()
}

async function allPages(path, key) {
  const first = await fetchJson(`${path}?limit=100&offset=0`)
  const items = [...first[key]]
  for (let offset = items.length; offset < first.total; offset += 100) {
    const page = await fetchJson(`${path}?limit=100&offset=${offset}`)
    items.push(...page[key])
  }
  return items
}

const text = (value) =>
  value == null
    ? 'NULL'
    : `convert_from(decode('${Buffer.from(String(value)).toString('base64')}', 'base64'), 'UTF8')`
const date = (value) => (value == null ? 'NULL' : `${text(value)}::date`)
const number = (value) => (value == null ? 'NULL' : String(Number(value)))
const boolean = (value) => (value ? 'TRUE' : 'FALSE')
const values = (row) => row.join(', ')
const numberKey = (raw) => raw.toLowerCase().normalize('NFD').replace(/[^a-z0-9]/g, '')

function person(fullName, role = 'Député') {
  const parts = fullName.trim().split(/\s+/)
  return {
    firstName: parts.shift() ?? fullName,
    lastName: parts.join(' ') || fullName,
    role,
  }
}

function runSql(sql) {
  const result = spawnSync(
    'docker',
    ['compose', '-f', 'compose.dev.yml', 'exec', '-T', 'postgres', 'psql', '-v', 'ON_ERROR_STOP=1', '-U', 'hemicycle', '-d', 'hemicycle_dev'],
    { cwd: process.cwd(), input: sql, encoding: 'utf8' },
  )
  if (result.status !== 0) throw new Error(result.stderr || result.stdout)
}

const dossier = await fetchJson(`/dossiers/${uid}`)
const scrutins = await allPages(`/dossiers/${uid}/scrutins`, 'scrutins')
const amendments = await allPages(`/dossiers/${uid}/amendements`, 'amendments')
const actions = await fetchJson(`/dossiers/${uid}/lecture-groupes`)
const details = []
for (const scrutin of scrutins) details.push(await fetchJson(`/scrutins/${scrutin.uid}`))

const finalByScrutin = new Map(
  actions.groups.flatMap((group) => group.final_votes).map((vote) => [vote.scrutin_uid, vote]),
)
const actors = new Map()
const rememberActor = (actorUid, fullName, role = 'Député') => {
  if (actorUid && fullName && !actors.has(actorUid)) actors.set(actorUid, { uid: actorUid, fullName, role })
}
details.forEach((scrutin) =>
  scrutin.groups.forEach((group) =>
    group.votes.forEach((vote) => rememberActor(vote.actor_uid, vote.full_name)),
  ),
)
amendments.forEach((amendment) => rememberActor(amendment.author_actor_uid, amendment.author_name))
dossier.initiators.forEach((initiator) => rememberActor(initiator.actor_uid, initiator.full_name, initiator.role ?? 'Député'))

const documentForAmendments = dossier.documents.find((document) => /PIONANR5L17B1560/.test(document.document_uid)) ?? dossier.documents[0]
if (!documentForAmendments) throw new Error('Le dossier ne comporte aucun document de rattachement')
const amendmentGroupUidByLabel = new Map(
  (await fetchJson(`/dossiers/${uid}/amendements?limit=1&offset=0`)).groups.map((group) => [group.label, group.uid]),
)

const statements = [
  'BEGIN;',
  `DELETE FROM amendments WHERE text_ref IN (SELECT document_uid FROM dossier_documents WHERE dossier_uid = ${text(uid)});`,
  `DELETE FROM scrutins WHERE dossier_uid = ${text(uid)};`,
  `DELETE FROM legislative_dossiers WHERE uid = ${text(uid)};`,
  `INSERT INTO legislative_dossiers (uid, title, procedure_label, last_activity_date, last_activity_label, score_progress, score_magnitude, score_momentum, score_total, committee, legislature, url, summary, deposit_date, outcome_kind, outcome_date, outcome_label, law_code, law_jo_date, law_legifrance_url, merged_into_uid)
   VALUES (${values([text(dossier.uid), text(dossier.title), text(dossier.procedure), date(dossier.last_activity_date), text(dossier.last_activity_label), number(dossier.score.progress), number(dossier.score.magnitude), number(dossier.score.momentum), number(dossier.score.total), text(dossier.committee), number(dossier.legislature), text(dossier.url), text(dossier.summary), date(dossier.deposit_date), text(dossier.outcome.kind), date(dossier.outcome.date), text(dossier.outcome.label), text(dossier.outcome.law_code), date(dossier.outcome.law_jo_date), text(dossier.outcome.legifrance_url), text(dossier.outcome.merged_into_uid)])})
   ON CONFLICT (uid) DO UPDATE SET title = EXCLUDED.title;`,
]

for (const act of dossier.acts) {
  statements.push(`INSERT INTO legislative_acts (dossier_uid, act_date, label, act_code) VALUES (${values([text(uid), date(act.date), text(act.label), text(act.code)])});`)
}
for (const document of dossier.documents) {
  statements.push(`INSERT INTO dossier_documents (dossier_uid, document_uid, title, short_title, doc_type, doc_date) VALUES (${values([text(uid), text(document.document_uid), text(document.title), text(document.short_title), text(document.doc_type), date(document.date)])});`)
}
for (const initiator of dossier.initiators) {
  statements.push(`INSERT INTO dossier_initiators (dossier_uid, full_name, actor_uid, group_uid, group_abbrev, group_label, membership_quality, reference_date, official_url, actor_role) VALUES (${values([text(uid), text(initiator.full_name), text(initiator.actor_uid), text(initiator.group?.uid), text(initiator.group?.abbrev), text(initiator.group?.label), text(initiator.group?.quality), date(initiator.reference_date), text(initiator.official_url), text(initiator.role)])});`)
}
for (const group of actions.groups) {
  statements.push(`INSERT INTO parliamentary_groups (uid, legislature, label, abbrev, color, start_date, end_date) VALUES (${values([text(group.uid), number(17), text(group.label), text(group.abbrev), text(group.color), date(group.start_date), date(group.end_date)])}) ON CONFLICT (uid) DO UPDATE SET label = EXCLUDED.label, abbrev = EXCLUDED.abbrev, color = EXCLUDED.color, start_date = EXCLUDED.start_date, end_date = EXCLUDED.end_date;`)
}
for (const actor of actors.values()) {
  const parsed = person(actor.fullName, actor.role)
  statements.push(`INSERT INTO actors (uid, civility, first_name, last_name, role) VALUES (${values([text(actor.uid), 'NULL', text(parsed.firstName), text(parsed.lastName), text(parsed.role)])}) ON CONFLICT (uid) DO UPDATE SET first_name = EXCLUDED.first_name, last_name = EXCLUDED.last_name, role = EXCLUDED.role;`)
}

for (const scrutin of details) {
  const synthesis = scrutin.synthesis
  statements.push(`INSERT INTO scrutins (uid, number, legislature, scrutin_date, session_ref, sitting_ref, place, ballot_type_code, ballot_type_label, majority_label, outcome_code, outcome_label, requester, subject, voters, expressed, required, announcement, votes_for, votes_against, abstentions, not_voting, voluntary_not_voting, dossier_uid, dossier_label) VALUES (${values([text(scrutin.uid), text(scrutin.number), number(scrutin.legislature), date(scrutin.date), text(scrutin.session_ref), text(scrutin.sitting_ref), text(scrutin.place), text(scrutin.ballot_type_code), text(scrutin.ballot_type_label), text(scrutin.majority_label), text(scrutin.outcome_code), text(scrutin.outcome_label), text(scrutin.requester), text(scrutin.subject), number(synthesis.voters), number(synthesis.expressed), number(synthesis.required), text(synthesis.announcement), number(synthesis.tally.votes_for), number(synthesis.tally.votes_against), number(synthesis.tally.abstentions), number(synthesis.tally.not_voting), number(synthesis.tally.voluntary_not_voting), text(scrutin.dossier_uid), text(scrutin.dossier_label)])});`)
  const final = finalByScrutin.get(scrutin.uid)
  const textKey = `local-${scrutin.uid}`
  statements.push(`INSERT INTO debated_texts (text_key, label) VALUES (${values([text(textKey), text(final?.text_label ?? dossier.title)])}) ON CONFLICT (text_key) DO UPDATE SET label = EXCLUDED.label;`)
  statements.push(`INSERT INTO scrutin_debated_texts (scrutin_uid, text_key) VALUES (${values([text(scrutin.uid), text(textKey)])}) ON CONFLICT DO NOTHING;`)
  for (const group of scrutin.groups) {
    if (!group.group_uid) continue
    statements.push(`INSERT INTO scrutin_group_tallies (scrutin_uid, group_uid, member_count, majority_position, votes_for, votes_against, abstentions, not_voting, voluntary_not_voting, origin) VALUES (${values([text(scrutin.uid), text(group.group_uid), number(group.member_count), text(group.majority_position), number(group.tally.votes_for), number(group.tally.votes_against), number(group.tally.abstentions), number(group.tally.not_voting), number(group.tally.voluntary_not_voting), text(group.origin)])});`)
    for (const vote of group.votes) statements.push(`INSERT INTO scrutin_votes (scrutin_uid, actor_uid, group_uid, position, cause_code, by_delegation, seat) VALUES (${values([text(scrutin.uid), text(vote.actor_uid), text(group.group_uid), text(vote.position), text(vote.cause_code), boolean(vote.by_delegation), number(vote.seat)])});`)
  }
  for (const correction of scrutin.corrections) statements.push(`INSERT INTO scrutin_vote_corrections (scrutin_uid, actor_uid, claimed_position, malfunction) VALUES (${values([text(scrutin.uid), text(correction.actor_uid), text(correction.claimed_position), boolean(correction.malfunction)])});`)
}

for (const amendment of amendments) {
  const authorKind = amendment.author_kind === 'institutional' ? 'institutional' : 'deputy'
  const authorGroupUid = amendment.author_group_origin === 'unknown'
    ? null
    : amendmentGroupUidByLabel.get(amendment.author_group_label) ?? null
  const authorGroupOrigin = authorGroupUid ? amendment.author_group_origin : 'unknown'
  statements.push(`INSERT INTO amendments (uid, legislature, number, number_key, text_ref, examination_ref, target_title, target_kind, author_kind, author_actor_uid, author_label, author_group_uid, author_group_origin, author_group_ambiguous, fate_code, fate_label, state_label, deposited_on, parent_uid, summary) VALUES (${values([text(amendment.uid), number(17), text(amendment.number), text(numberKey(amendment.number)), text(documentForAmendments.document_uid), 'NULL', text(amendment.target_title), text(amendment.target_kind), text(authorKind), text(authorKind === 'deputy' ? amendment.author_actor_uid : null), text(authorKind === 'institutional' ? amendment.author_name : null), text(authorGroupUid), text(authorGroupOrigin), boolean(amendment.author_group_ambiguous), text(amendment.fate_code), text(amendment.fate_label), text(amendment.state_label), date(amendment.deposited_on), 'NULL', text(amendment.summary)])});`)
}
statements.push('COMMIT;')
runSql(statements.join('\n'))
console.log(JSON.stringify({ dossier: uid, scrutins: details.length, amendments: amendments.length, actors: actors.size }))
