import { Sequence, spring, useCurrentFrame, useVideoConfig, interpolate } from 'remotion'
import { leadingPosition, type LeadingPosition } from '../../../frontend/src/types/scrutins'
import { Soundtrack } from '../components'
import {
  BigCount,
  capitalize,
  CREAM,
  Fade,
  Hemicycle,
  Logo,
  NAVY,
  NOT_VOTING,
  ON_NAVY,
  outcomeColor,
  PAPER,
  REEL_SAFE,
  SEAT_EMPTY,
  Safe,
  useEnter,
  useFonts,
} from '../motion'
import { COLORS } from '../theme'
import { GROUPS, HEMICYCLE, HOOK, PERIOD, scenesOf, VOTES_OUTRO } from '../timeline'
import type { ReelFinalVoteDto, ReelProps, ReelResponse, ScrutinDetailDto } from '../types'

const MOST_FOLLOWED_CRITERION = 'Critère : le scrutin qui compte le plus de votants (pour, contre, abstention).'

/** Rang d'affichage des positions : les groupes sont rangés par position (RM-11). */
const POSITION_ORDER: (LeadingPosition | null)[] = ['for', 'against', 'abstention', null]
const POSITION_LABELS: Record<LeadingPosition, string> = { for: 'pour', against: 'contre', abstention: 'abstention' }
const POSITION_COLORS: Record<LeadingPosition, string> = { for: COLORS.adopted, against: COLORS.rejected, abstention: COLORS.other }

function detailOf(details: ReelProps['details'], uid: string): ScrutinDetailDto {
  const detail = details[uid]
  if (!detail) throw new Error(`Détail du scrutin ${uid} absent des propriétés`)
  return detail
}

const tallySize = (t: ScrutinDetailDto['groups'][number]['tally']) => t.votes_for + t.votes_against + t.abstentions + t.not_voting

/** Groupes qui ont pris part au vote, rangés par position, dans l'ordre publié à position égale. */
function groupsByPosition(detail: ScrutinDetailDto) {
  return detail.groups
    .filter((g) => tallySize(g.tally) > 0)
    .map((g) => ({ group: g, position: leadingPosition(g.tally) }))
    .sort((a, b) => POSITION_ORDER.indexOf(a.position) - POSITION_ORDER.indexOf(b.position))
}

/** Taille d'un nombre selon son nombre de chiffres, pour qu'il tienne dans sa colonne. */
function fit(base: number, ...values: number[]) {
  const digits = Math.max(...values.map((value) => value.toLocaleString('fr-FR').length))
  return digits <= 2 ? base : Math.round((base * 2.2) / (digits + 0.2))
}

function plural(count: number, one: string, many: string) {
  return count > 1 ? many : one
}

function dayLabel(date: string) {
  const day = new Date(`${date}T12:00:00`)
  const weekday = day.toLocaleDateString('fr-FR', { weekday: 'short' }).replace('.', '')
  return `${capitalize(weekday)}. ${day.getDate()}`
}

/* ------------------------------------------------------------------ accroche */

function Hook({ reel, detail }: { reel: ReelResponse; detail: ScrutinDetailDto }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const mf = reel.most_followed!
  const adopted = detail.outcome_code.startsWith('adopt')
  const stamp = spring({ frame: frame - 6, fps, config: { damping: 12, stiffness: 160 } })
  const date = useEnter(0, 30)
  const dossier = useEnter(18)
  const subject = useEnter(30)
  const numbers = useEnter(42, 120)
  const items = [
    { value: mf.tally.votes_for, label: 'pour' },
    { value: mf.tally.votes_against, label: 'contre' },
  ]
  if (!adopted) items.reverse()
  return (
    <Safe background={outcomeColor(detail.outcome_code)} color="#ffffff">
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-between', gap: 36 }}>
        <div style={{ ...date, display: 'flex', flexDirection: 'column', gap: 6 }}>
          <span style={{ fontSize: 42, fontWeight: 600, opacity: 0.85 }}>Assemblée nationale · {reel.period.label}</span>
          <span style={{ fontSize: 42, fontWeight: 800 }}>Le scrutin le plus suivi</span>
        </div>
        <div
          style={{
            alignSelf: 'flex-start',
            transform: `scale(${0.4 + 0.6 * stamp}) rotate(${(1 - stamp) * -8 - 3}deg)`,
            transformOrigin: 'left center',
            opacity: Math.min(1, stamp * 2),
            border: '10px solid #ffffff',
            borderRadius: 22,
            padding: '4px 34px',
            fontSize: 150,
            fontWeight: 900,
            letterSpacing: 4,
            textTransform: 'uppercase',
          }}
        >
          {capitalize(detail.outcome_code)}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
          {detail.dossier_label && (
            <div style={{ ...dossier, fontSize: detail.dossier_label.length > 60 ? 70 : 90, fontWeight: 800, lineHeight: 1.02, letterSpacing: -2 }}>{detail.dossier_label}</div>
          )}
          <div
            style={{
              ...subject,
              fontSize: detail.dossier_label ? 40 : 68,
              fontWeight: detail.dossier_label ? 400 : 800,
              lineHeight: 1.25,
              opacity: subject.opacity * 0.92,
              display: '-webkit-box',
              WebkitLineClamp: detail.dossier_label ? 3 : 5,
              WebkitBoxOrient: 'vertical',
              overflow: 'hidden',
            }}
          >
            {capitalize(mf.subject)}
          </div>
        </div>
        <div style={{ ...numbers, display: 'flex', flexDirection: 'column', gap: 18 }}>
          <div style={{ display: 'flex', alignItems: 'flex-end', gap: 50 }}>
            {items.map((item, index) => (
              <div key={item.label} style={{ display: 'flex', flexDirection: 'column', opacity: index === 0 ? 1 : 0.72 }}>
                <span style={{ fontSize: fit(index === 0 ? 310 : 200, item.value), fontWeight: 900, lineHeight: 0.85, letterSpacing: -6, fontVariantNumeric: 'tabular-nums' }}>
                  <BigCount value={item.value} start={44} />
                </span>
                <span style={{ fontSize: 62, fontWeight: 800, marginTop: 10 }}>{item.label}</span>
              </div>
            ))}
          </div>
          <span style={{ fontSize: 32, lineHeight: 1.3, opacity: 0.85 }}>{MOST_FOLLOWED_CRITERION}</span>
        </div>
      </div>
    </Safe>
  )
}

/* ---------------------------------------------------------------- hémicycle */

const FLIP = 95

function HemicycleScene({ detail, tally }: { detail: ScrutinDetailDto; tally: NonNullable<ReelResponse['most_followed']>['tally'] }) {
  const frame = useCurrentFrame()
  const title = useEnter(0, 40)
  const chipsOut = interpolate(frame, [FLIP - 8, FLIP], [1, 0], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
  const counters = useEnter(FLIP + 4, 80)
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ ...title, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <div style={{ fontSize: 92, fontWeight: 900, letterSpacing: -2, lineHeight: 1 }}>Qui a voté quoi ?</div>
        <div style={{ fontSize: 42, lineHeight: 1.3, color: COLORS.inkSoft }}>
          {frame < FLIP ? 'Chaque point : un député qui a pris part au vote, à sa place.' : 'Et voici comment chacun a voté.'}
        </div>
      </div>
      <div style={{ marginTop: 90, marginLeft: -REEL_SAFE.left + 12, marginRight: -REEL_SAFE.right + 12 }}>
        <Hemicycle detail={detail} light={16} flip={FLIP} />
      </div>
      <div style={{ flex: 1, position: 'relative', marginTop: 40 }}>
        <div style={{ position: 'absolute', inset: 0, display: 'flex', flexWrap: 'wrap', alignContent: 'flex-start', gap: '22px 34px', opacity: chipsOut }}>
          {detail.groups
            .filter((g) => tallySize(g.tally) > 0)
            .map((g, index) => {
              const enter = interpolate(frame, [20 + index * 4, 32 + index * 4], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
              return (
                <div key={g.group_uid} style={{ display: 'flex', alignItems: 'center', gap: 14, opacity: enter, fontSize: 50, fontWeight: 800 }}>
                  <span style={{ width: 36, height: 36, borderRadius: 18, background: g.color ?? NOT_VOTING }} />
                  {g.abbrev}
                </div>
              )
            })}
        </div>
        <div style={{ ...counters, position: 'absolute', inset: 0, display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
          {[
            { label: 'pour', value: tally.votes_for, color: COLORS.adopted },
            { label: 'contre', value: tally.votes_against, color: COLORS.rejected },
            { label: plural(tally.abstentions, 'abstention', 'abstentions'), value: tally.abstentions, color: COLORS.other },
          ].map((item) => (
            <div key={item.color} style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              <span style={{ fontSize: fit(205, tally.votes_for, tally.votes_against, tally.abstentions), fontWeight: 900, lineHeight: 0.9, letterSpacing: -6, color: item.color, fontVariantNumeric: 'tabular-nums' }}>
                <BigCount value={item.value} start={FLIP + 6} />
              </span>
              <span style={{ fontSize: 56, fontWeight: 800, color: item.color }}>{item.label}</span>
            </div>
          ))}
        </div>
      </div>
    </Safe>
  )
}

/* ---------------------------------------------------------------- groupes */

function GroupsScene({ detail }: { detail: ScrutinDetailDto }) {
  const frame = useCurrentFrame()
  const title = useEnter(0, 40)
  const groups = groupsByPosition(detail)
  const largest = Math.max(...groups.map(({ group }) => tallySize(group.tally)))
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ ...title, fontSize: 92, fontWeight: 900, letterSpacing: -2, lineHeight: 1 }}>Groupe par groupe</div>
      <div style={{ flex: 1, marginTop: 40, display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        {groups.map(({ group: g, position }, index) => {
          const start = 8 + index * 6
          const enter = interpolate(frame, [start, start + 14], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
          const grow = interpolate(frame, [start + 6, start + 30], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
          const parts = [
            { value: g.tally.votes_for, color: COLORS.adopted },
            { value: g.tally.votes_against, color: COLORS.rejected },
            { value: g.tally.abstentions, color: COLORS.other },
            { value: g.tally.not_voting, color: NOT_VOTING },
          ]
          const count = position === null ? null : { for: g.tally.votes_for, against: g.tally.votes_against, abstention: g.tally.abstentions }[position]
          return (
            <div key={g.group_uid} style={{ display: 'flex', flexDirection: 'column', gap: 10, opacity: enter, transform: `translateX(${(1 - enter) * -80}px)` }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 18 }}>
                <span style={{ width: 36, height: 36, borderRadius: 18, background: g.color ?? NOT_VOTING, flexShrink: 0 }} />
                <span style={{ flex: 1, fontSize: 58, fontWeight: 900 }}>{g.abbrev}</span>
                <span style={{ fontSize: 58, fontWeight: 900, color: position ? POSITION_COLORS[position] : COLORS.inkSoft }}>
                  {position ? `${count} ${POSITION_LABELS[position]}` : 'sans majorité'}
                </span>
              </div>
              <div style={{ height: 30, display: 'flex', borderRadius: 15, overflow: 'hidden', background: SEAT_EMPTY }}>
                {parts.map((p) => (
                  <div key={p.color} style={{ width: `${((p.value / largest) * 100 * grow).toFixed(2)}%`, background: p.color }} />
                ))}
              </div>
            </div>
          )
        })}
      </div>
    </Safe>
  )
}

/* ---------------------------------------------------------------- période */

const PERIOD_KICKER: Record<ReelResponse['kind'], string> = {
  veille: 'Toute la journée',
  semaine: 'Toute la semaine',
  lundi: 'Sur la période',
}

function PeriodScene({ reel }: { reel: ReelResponse }) {
  const frame = useCurrentFrame()
  const totals = reel.totals ?? { votes: 0, adopted: 0, rejected: 0, others: [] }
  const week = reel.days.length > 1
  const label = useEnter(0, 40)
  const big = useEnter(8, 100)
  const split = useEnter(28, 100)
  const grow = interpolate(frame, [34, 64], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
  const notes = [
    totals.others.length > 0 ? `Dont ${totals.others.map((other) => `${other.count} ${other.label}`).join(', ')}.` : null,
    reel.empty_week_note,
  ].filter((note): note is string => Boolean(note))
  return (
    <Safe background={NAVY} color="#ffffff">
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div style={{ ...label, display: 'flex', flexDirection: 'column', gap: 10 }}>
          <span style={{ fontSize: 50, fontWeight: 700, color: CREAM }}>{PERIOD_KICKER[reel.kind]}</span>
          <span style={{ fontSize: week ? 70 : 92, fontWeight: 900, letterSpacing: -1.5, lineHeight: 1.05 }}>{reel.period.label}</span>
        </div>
        <div style={{ ...big, display: 'flex', flexDirection: 'column' }}>
          <span style={{ fontSize: week ? 360 : 520, fontWeight: 900, lineHeight: 0.85, letterSpacing: week ? -12 : -18 }}>
            <BigCount value={totals.votes} start={10} />
          </span>
          <span style={{ fontSize: 74, fontWeight: 800, marginTop: 10 }}>{plural(totals.votes, 'vote public', 'votes publics')}</span>
        </div>
        {week && (
          <div style={{ ...split, display: 'flex', gap: 14 }}>
            {reel.days.map((day) => (
              <div key={day.date} style={{ flex: 1, borderRadius: 20, padding: '12px 0', textAlign: 'center', background: 'rgba(255,255,255,0.08)' }}>
                <div style={{ fontSize: 30, color: CREAM }}>{dayLabel(day.date)}</div>
                <div style={{ fontSize: 56, fontWeight: 800 }}>{day.votes}</div>
              </div>
            ))}
          </div>
        )}
        <div style={{ ...split, display: 'flex', flexDirection: 'column', gap: 26 }}>
          <div style={{ display: 'flex', gap: 70 }}>
            {[
              { value: totals.adopted, label: plural(totals.adopted, 'adopté', 'adoptés'), color: ON_NAVY.adopted },
              { value: totals.rejected, label: plural(totals.rejected, 'rejeté', 'rejetés'), color: ON_NAVY.rejected },
            ].map((item) => (
              <div key={item.color} style={{ display: 'flex', flexDirection: 'column' }}>
                <span style={{ fontSize: week ? 170 : 210, fontWeight: 900, lineHeight: 0.9, color: item.color }}>
                  <BigCount value={item.value} start={30} />
                </span>
                <span style={{ fontSize: 58, fontWeight: 700 }}>{item.label}</span>
              </div>
            ))}
          </div>
          {totals.votes > 0 && (
            <div style={{ display: 'flex', height: 56, borderRadius: 28, overflow: 'hidden', background: 'rgba(255,255,255,0.12)' }}>
              <div style={{ width: `${(totals.adopted / totals.votes) * 100 * grow}%`, background: ON_NAVY.adopted }} />
              <div style={{ width: `${(totals.rejected / totals.votes) * 100 * grow}%`, background: ON_NAVY.rejected }} />
            </div>
          )}
          {notes.map((note) => (
            <span key={note} style={{ fontSize: 34, lineHeight: 1.3, color: 'rgba(255,255,255,0.75)' }}>
              {note}
            </span>
          ))}
        </div>
      </div>
    </Safe>
  )
}

/* ------------------------------------------------------- votes sur l'ensemble */

function Pill({ code, size }: { code: string; size: number }) {
  return (
    <span
      style={{
        alignSelf: 'flex-start',
        background: outcomeColor(code),
        color: '#ffffff',
        borderRadius: size * 0.3,
        padding: `${size * 0.08}px ${size * 0.4}px`,
        fontSize: size,
        fontWeight: 900,
        letterSpacing: 2,
        textTransform: 'uppercase',
      }}
    >
      {capitalize(code)}
    </span>
  )
}

/** Décompte de toute l'Assemblée publié dans la synthèse officielle (RM-03). */
function Tally({ detail, size, start }: { detail: ScrutinDetailDto; size: number; start: number }) {
  const t = detail.synthesis.tally
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
      {[
        { value: t.votes_for, label: 'pour', color: COLORS.adopted },
        { value: t.votes_against, label: 'contre', color: COLORS.rejected },
        { value: t.abstentions, label: plural(t.abstentions, 'abstention', 'abstentions'), color: COLORS.other },
      ].map((item) => (
        <div key={item.color} style={{ display: 'flex', flexDirection: 'column' }}>
          <span style={{ fontSize: size, fontWeight: 900, lineHeight: 0.9, letterSpacing: -3, color: item.color, fontVariantNumeric: 'tabular-nums' }}>
            <BigCount value={item.value} start={start} duration={20} />
          </span>
          <span style={{ fontSize: size * 0.32, fontWeight: 800, color: item.color }}>{item.label}</span>
        </div>
      ))}
    </div>
  )
}

/** Groupes rangés par position, à leurs couleurs (RM-03, RM-11). */
function GroupsByPosition({ detail, size }: { detail: ScrutinDetailDto; size: number }) {
  const groups = groupsByPosition(detail)
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: size * 0.4 }}>
      {POSITION_ORDER.map((position) => {
        const members = groups.filter((g) => g.position === position)
        if (members.length === 0) return null
        return (
          <div key={position ?? 'none'} style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: `${size * 0.3}px ${size * 0.6}px` }}>
            <span style={{ fontSize: size, fontWeight: 900, color: position ? POSITION_COLORS[position] : COLORS.inkSoft, minWidth: size * 5 }}>
              {position ? capitalize(POSITION_LABELS[position]) : 'Sans majorité'}
            </span>
            {members.map(({ group: g }) => (
              <span key={g.group_uid} style={{ display: 'flex', alignItems: 'center', gap: size * 0.3, fontSize: size, fontWeight: 800 }}>
                <span style={{ width: size * 0.7, height: size * 0.7, borderRadius: size, background: g.color ?? NOT_VOTING }} />
                {g.abbrev}
              </span>
            ))}
          </div>
        )
      })}
    </div>
  )
}

function FinalVoteCard({ vote, detail, index, count }: { vote: ReelFinalVoteDto; detail: ScrutinDetailDto; index: number; count: number }) {
  const head = useEnter(0, 30)
  const title = useEnter(4, 40)
  const rest = useEnter(14, 40)
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div style={{ ...head, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ fontSize: 44, fontWeight: 800, color: COLORS.accent }}>Vote sur l'ensemble</span>
          <span style={{ fontSize: 42, fontWeight: 700, color: COLORS.inkFaint }}>
            {index + 1} / {count}
          </span>
        </div>
        <div style={{ ...title, display: 'flex', flexDirection: 'column', gap: 22 }}>
          <Pill code={vote.outcome_code} size={66} />
          <div
            style={{
              fontSize: 58,
              fontWeight: 800,
              lineHeight: 1.1,
              letterSpacing: -1,
              display: '-webkit-box',
              WebkitLineClamp: 6,
              WebkitBoxOrient: 'vertical',
              overflow: 'hidden',
            }}
          >
            {capitalize(vote.text_label)}
          </div>
        </div>
        <div style={{ marginLeft: 40, marginRight: 40 }}>
          <Hemicycle detail={detail} light={4} flip={0} stagger={0.4} />
        </div>
        <div style={{ ...rest, display: 'flex', flexDirection: 'column', gap: 30 }}>
          <Tally detail={detail} size={130} start={10} />
          <GroupsByPosition detail={detail} size={36} />
        </div>
      </div>
    </Safe>
  )
}

/** Plusieurs votes sur une carte, quand ils sont trop nombreux pour une carte chacun (T7). */
function FinalVotesList({ votes, details }: { votes: { vote: ReelFinalVoteDto; index: number }[]; details: ReelProps['details'] }) {
  const style = useEnter(0, 30)
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ ...style, flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-around' }}>
        <span style={{ fontSize: 44, fontWeight: 800, color: COLORS.accent }}>Votes sur l'ensemble</span>
        {votes.map(({ vote }) => {
          const detail = detailOf(details, vote.scrutin_uid)
          return (
            <div key={vote.scrutin_uid} style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              <Pill code={vote.outcome_code} size={44} />
              <div style={{ fontSize: 48, fontWeight: 800, lineHeight: 1.12, display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
                {capitalize(vote.text_label)}
              </div>
              <Tally detail={detail} size={90} start={6} />
              <GroupsByPosition detail={detail} size={30} />
            </div>
          )
        })}
      </div>
    </Safe>
  )
}

/* ------------------------------------------------------------------- fin */

function Outro({ site, linkPath, note }: { site: string; linkPath: string | null; note: string | null }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const logo = spring({ frame, fps, config: { damping: 14 } })
  const name = useEnter(8, 40)
  const tagline = useEnter(18, 40)
  const url = useEnter(30, 40)
  return (
    <Safe background={NAVY} color="#ffffff">
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 30 }}>
          <div style={{ transform: `scale(${logo})`, transformOrigin: 'left bottom' }}>
            <Logo height={270} />
          </div>
          <div style={{ ...name, fontSize: 140, fontWeight: 900, letterSpacing: -4, lineHeight: 1 }}>
            QuiVote<span style={{ color: '#60a5fa' }}>Quoi</span>
          </div>
        </div>
        <div style={{ ...tagline, fontSize: 84, fontWeight: 800, lineHeight: 1.08, letterSpacing: -1 }}>
          Tous les votes de l'Assemblée, groupe par groupe.
        </div>
        <div style={{ ...url, display: 'flex', flexDirection: 'column', gap: 12 }}>
          <span style={{ fontSize: 46, fontWeight: 600, color: 'rgba(255,255,255,0.7)' }}>Tous les chiffres et leurs sources</span>
          <span style={{ fontSize: 112, fontWeight: 900, letterSpacing: -3, color: CREAM, lineHeight: 1 }}>{site}</span>
          {linkPath && <span style={{ fontSize: 38, color: 'rgba(255,255,255,0.7)', wordBreak: 'break-all' }}>{linkPath}</span>}
          {note && <span style={{ fontSize: 28, lineHeight: 1.3, color: 'rgba(255,255,255,0.7)' }}>{note}</span>}
        </div>
      </div>
    </Safe>
  )
}

/* ------------------------------------------------------------- composition */

/**
 * Reel de votes, de la veille ou de la semaine (RM-03, RM-04) : le scrutin le
 * plus suivi ouvre (sort, hémicycle, groupes rangés par position), puis les
 * nombres de la période et chaque vote sur l'ensemble dans l'ordre de l'API,
 * enfin l'adresse du site. Aucun vote n'est retiré, aucun chiffre n'est
 * calculé ici (RM-10) : nombres et décomptes sont ceux de l'API.
 */
export function Votes(props: ReelProps) {
  useFonts()
  const { reel, details, site } = props
  const scenes = scenesOf(props)
  const mostFollowed = reel.most_followed ? detailOf(details, reel.most_followed.uid) : null
  const periodFrom = scenes.mostFollowed
  const cardsFrom = periodFrom + PERIOD
  const outroFrom = cardsFrom + scenes.cards.groups.length * scenes.cards.each

  return (
    <>
      <Soundtrack audio={props.audio} />
      {reel.most_followed && mostFollowed && (
        <>
          <Sequence durationInFrames={HOOK}>
            <Fade length={HOOK}>
              <Hook reel={reel} detail={mostFollowed} />
            </Fade>
          </Sequence>
          <Sequence from={HOOK} durationInFrames={HEMICYCLE}>
            <Fade length={HEMICYCLE}>
              <HemicycleScene detail={mostFollowed} tally={reel.most_followed.tally} />
            </Fade>
          </Sequence>
          <Sequence from={HOOK + HEMICYCLE} durationInFrames={GROUPS}>
            <Fade length={GROUPS}>
              <GroupsScene detail={mostFollowed} />
            </Fade>
          </Sequence>
        </>
      )}
      <Sequence from={periodFrom} durationInFrames={PERIOD}>
        <Fade length={PERIOD}>
          <PeriodScene reel={reel} />
        </Fade>
      </Sequence>
      {scenes.cards.groups.map((indexes, card) => (
        <Sequence key={indexes.join('-')} from={cardsFrom + card * scenes.cards.each} durationInFrames={scenes.cards.each}>
          <Fade length={scenes.cards.each}>
            {indexes.length === 1 ? (
              <FinalVoteCard
                vote={reel.final_votes[indexes[0]]}
                detail={detailOf(details, reel.final_votes[indexes[0]].scrutin_uid)}
                index={indexes[0]}
                count={reel.final_votes.length}
              />
            ) : (
              <FinalVotesList votes={indexes.map((i) => ({ vote: reel.final_votes[i], index: i }))} details={details} />
            )}
          </Fade>
        </Sequence>
      ))}
      <Sequence from={outroFrom} durationInFrames={VOTES_OUTRO}>
        <Fade length={VOTES_OUTRO + 8}>
          <Outro site={site} linkPath={reel.link_path} note={reel.reconstructed_note} />
        </Fade>
      </Sequence>
    </>
  )
}
