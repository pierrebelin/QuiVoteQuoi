import { Easing, Sequence, spring, useCurrentFrame, useVideoConfig, interpolate } from 'remotion'
import { leadingPosition, type LeadingPosition } from '../../../frontend/src/types/scrutins'
import { Soundtrack } from '../components'
import {
  BigCount,
  BrandOutro,
  capitalize,
  CREAM,
  arcLabels,
  Hemicycle,
  NAVY,
  NOT_VOTING,
  ON_NAVY,
  outcomeColor,
  PAPER,
  Push,
  SEAT_EMPTY,
  Safe,
  useEnter,
  useFonts,
} from '../motion'
import { COLORS, HEIGHT } from '../theme'
import { HOOK, PERIOD, PUSH, scenesOf, VOTES_OUTRO, WHO_VOTED } from '../timeline'
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
  const stamp = spring({ frame: frame - 4, fps, config: { damping: 12, stiffness: 160 } })
  const kicker = useEnter(0, 30)
  const dossier = useEnter(14)
  const subject = useEnter(24)
  const label = detail.dossier_label
  return (
    <Safe background={outcomeColor(detail.outcome_code)} color="#ffffff">
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 56 }}>
        <span style={{ ...kicker, fontSize: 44, fontWeight: 800 }}>
          Le scrutin le plus suivi · <span style={{ opacity: 0.8 }}>{reel.period.label}</span>
        </span>
        <div
          style={{
            alignSelf: 'flex-start',
            transform: `scale(${0.4 + 0.6 * stamp}) rotate(${(1 - stamp) * -8 - 3}deg)`,
            transformOrigin: 'left center',
            opacity: Math.min(1, stamp * 2),
            border: '12px solid #ffffff',
            borderRadius: 26,
            padding: '6px 40px',
            fontSize: 190,
            fontWeight: 900,
            letterSpacing: 4,
            textTransform: 'uppercase',
          }}
        >
          {capitalize(detail.outcome_code)}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 28 }}>
          {label && (
            <div style={{ ...dossier, fontSize: label.length > 60 ? 76 : 96, fontWeight: 900, lineHeight: 1.02, letterSpacing: -2 }}>{label}</div>
          )}
          <div
            style={{
              ...subject,
              fontSize: label ? 38 : 68,
              fontWeight: label ? 500 : 800,
              lineHeight: 1.25,
              opacity: subject.opacity * 0.85,
              display: '-webkit-box',
              WebkitLineClamp: label ? 2 : 5,
              WebkitBoxOrient: 'vertical',
              overflow: 'hidden',
            }}
          >
            {capitalize(mf.subject)}
          </div>
        </div>
      </div>
    </Safe>
  )
}

/* ---------------------------------------------------------------- hémicycle */

/**
 * Allumage aux couleurs des groupes, chaque sigle posé au droit de ses sièges ;
 * bascule vers les positions et barre du décompte ; repli de l'hémicycle sous
 * lequel entrent les groupes, rangés par position.
 */
const FLIP = 80
const FOLD = 160
/** Descente de l'hémicycle et du décompte avant le repli, pour les centrer dans l'écran vide. */
const CENTRE_SHIFT = 210

function WhoVotedScene({ detail, tally }: { detail: ScrutinDetailDto; tally: NonNullable<ReelResponse['most_followed']>['tally'] }) {
  const frame = useCurrentFrame()
  const { fps, height } = useVideoConfig()
  // Post en 4:5 : moins de hauteur, l'hémicycle et les lignes des groupes rétrécissent pour que tous tiennent.
  const post = height < HEIGHT
  const rowSize = post ? 30 : 38
  const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const
  const kicker = useEnter(0, 24)
  const labelsOut = interpolate(frame, [FLIP - 6, FLIP + 4], [1, 0], clamp)
  const flip = spring({ frame: frame - FLIP, fps, config: { damping: 200 }, durationInFrames: 30 })
  const fold = spring({ frame: frame - FOLD, fps, config: { damping: 200 }, durationInFrames: 26 })
  const groups = groupsByPosition(detail)
  const largest = Math.max(...groups.map(({ group }) => tallySize(group.tally)))
  const items = [
    { label: 'pour', value: tally.votes_for, color: COLORS.adopted },
    { label: 'contre', value: tally.votes_against, color: COLORS.rejected },
    { label: plural(tally.abstentions, 'abstention', 'abstentions'), value: tally.abstentions, color: COLORS.other },
  ]
  const expressed = items.reduce((sum, item) => sum + item.value, 0) || 1
  const countSize = interpolate(fold, [0, 1], [fit(170, ...items.map((item) => item.value)), 96])
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ ...kicker, display: 'flex', alignItems: 'center', gap: 24 }}>
        <span style={{ fontSize: 34, fontWeight: 800, letterSpacing: 5, textTransform: 'uppercase' }}>Qui a voté quoi</span>
        <span style={{ flex: 1, height: 3, background: NAVY, opacity: 0.85 }} />
      </div>
      <div
        style={{
          flexShrink: 0,
          position: 'relative',
          alignSelf: 'center',
          marginTop: 44,
          width: `${interpolate(fold, [0, 1], [70, post ? 42 : 52])}%`,
          transform: `translateY(${(1 - fold) * CENTRE_SHIFT + (1 - flip) * 130}px)`,
        }}
      >
        <Hemicycle detail={detail} light={8} flip={FLIP} />
        {arcLabels(detail).map((label, index) => {
          const pop = spring({ frame: frame - 18 - index * 3, fps, config: { damping: 14, stiffness: 170 } })
          return (
            <div
              key={label.uid}
              style={{
                position: 'absolute',
                left: `${label.left}%`,
                top: `${label.top}%`,
                transform: `translate(${-50 + 50 * Math.cos(label.angle)}%, ${-50 - 45 * Math.sin(label.angle)}%) scale(${pop})`,
                opacity: Math.min(pop, labelsOut),
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                fontSize: 30,
                fontWeight: 800,
                whiteSpace: 'nowrap',
              }}
            >
              <span style={{ width: 18, height: 18, borderRadius: 9, background: label.color }} />
              {label.abbrev}
            </div>
          )
        })}
      </div>
      <div style={{ flexShrink: 0, marginTop: interpolate(fold, [0, 1], [100, 48]), opacity: flip, transform: `translateY(${(1 - flip) * 40 + (1 - fold) * CENTRE_SHIFT}px)` }}>
        <div style={{ display: 'flex', height: interpolate(fold, [0, 1], [26, 16]), borderRadius: 13, overflow: 'hidden', background: SEAT_EMPTY }}>
          {items.map((item) => (
            <div key={item.color} style={{ width: `${(item.value / expressed) * 100 * flip}%`, background: item.color }} />
          ))}
        </div>
        <div style={{ display: 'flex', gap: 72, marginTop: interpolate(fold, [0, 1], [36, 20]) }}>
          {items.map((item) => (
            <div key={item.color} style={{ display: 'flex', flexDirection: 'column' }}>
              <span style={{ fontSize: countSize, fontWeight: 900, lineHeight: 0.9, letterSpacing: -4, color: item.color, fontVariantNumeric: 'tabular-nums' }}>
                <BigCount value={item.value} start={FLIP + 4} />
              </span>
              <span style={{ fontSize: interpolate(fold, [0, 1], [44, 32]), fontWeight: 800, color: item.color }}>{item.label}</span>
            </div>
          ))}
        </div>
      </div>
      <div style={{ flex: 1, minHeight: 0, overflow: 'hidden', marginTop: post ? 36 : 60, display: 'flex', flexDirection: 'column', justifyContent: 'space-between', opacity: fold }}>
        {groups.map(({ group: g, position }, index) => {
          const start = FOLD + 10 + index * 4
          const enter = interpolate(frame, [start, start + 12], [0, 1], clamp)
          const grow = interpolate(frame, [start + 4, start + 26], [0, 1], { ...clamp, easing: Easing.out(Easing.cubic) })
          const parts = [
            { value: g.tally.votes_for, color: COLORS.adopted },
            { value: g.tally.votes_against, color: COLORS.rejected },
            { value: g.tally.abstentions, color: COLORS.other },
            { value: g.tally.not_voting, color: NOT_VOTING },
          ]
          const count = position === null ? null : { for: g.tally.votes_for, against: g.tally.votes_against, abstention: g.tally.abstentions }[position]
          return (
            <div key={g.group_uid} style={{ display: 'flex', alignItems: 'center', gap: 16, opacity: enter, transform: `translateX(${(1 - enter) * -60}px)` }}>
              <span style={{ width: 26, height: 26, borderRadius: 13, background: g.color ?? NOT_VOTING, flexShrink: 0 }} />
              <span style={{ width: 180, fontSize: rowSize, fontWeight: 800, flexShrink: 0 }}>{g.abbrev}</span>
              <div style={{ flex: 1, height: 18, display: 'flex', borderRadius: 9, overflow: 'hidden', background: SEAT_EMPTY }}>
                {parts.map((p) => (
                  <div key={p.color} style={{ width: `${((p.value / largest) * 100 * grow).toFixed(2)}%`, background: p.color }} />
                ))}
              </div>
              <span style={{ minWidth: 200, textAlign: 'right', fontSize: rowSize, fontWeight: 800, color: position ? POSITION_COLORS[position] : COLORS.inkSoft, flexShrink: 0 }}>
                {position ? `${count} ${position === 'abstention' ? plural(count ?? 0, 'abstention', 'abstentions') : POSITION_LABELS[position]}` : 'sans majorité'}
              </span>
            </div>
          )
        })}
      </div>
      <div style={{ ...kicker, flexShrink: 0, marginTop: post ? 24 : 40, fontSize: 26, lineHeight: 1.35, color: COLORS.inkFaint }}>{MOST_FOLLOWED_CRITERION}</div>
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
              WebkitLineClamp: 4,
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
          <Sequence durationInFrames={HOOK + PUSH}>
            <Push length={HOOK} overlap={PUSH} first>
              <Hook reel={reel} detail={mostFollowed} />
            </Push>
          </Sequence>
          <Sequence from={HOOK} durationInFrames={WHO_VOTED + PUSH}>
            <Push length={WHO_VOTED} overlap={PUSH}>
              <WhoVotedScene detail={mostFollowed} tally={reel.most_followed.tally} />
            </Push>
          </Sequence>
        </>
      )}
      <Sequence from={periodFrom} durationInFrames={PERIOD + PUSH}>
        <Push length={PERIOD} overlap={PUSH} first={periodFrom === 0}>
          <PeriodScene reel={reel} />
        </Push>
      </Sequence>
      {scenes.cards.groups.map((indexes, card) => (
        <Sequence key={indexes.join('-')} from={cardsFrom + card * scenes.cards.each} durationInFrames={scenes.cards.each + PUSH}>
          <Push length={scenes.cards.each} overlap={PUSH} axis={card === 0 ? 'y' : 'x'}>
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
          </Push>
        </Sequence>
      ))}
      <Sequence from={outroFrom} durationInFrames={VOTES_OUTRO}>
        <Push length={VOTES_OUTRO} overlap={PUSH} last>
          <BrandOutro site={site} linkPath={reel.link_path} note={reel.reconstructed_note} />
        </Push>
      </Sequence>
    </>
  )
}
