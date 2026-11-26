import type { CSSProperties, ReactNode } from 'react'
import { Easing, Sequence, spring, useCurrentFrame, useVideoConfig, interpolate } from 'remotion'
import { leadingPosition, type LeadingPosition } from '../../../frontend/src/types/scrutins'
import { Soundtrack } from '../components'
import {
  BigCount,
  BrandOutro,
  capitalize,
  FADE,
  Fade,
  Hemicycle,
  Kicker,
  Night,
  OutcomeIcon,
  outcomeTone,
  PLAN_RATIO,
  POSITION_COLORS,
  POSITION_SOFT,
  Stage,
  TopBar,
  useEnter,
  useFonts,
  useLayout,
} from '../motion'
import { FONT_DISPLAY, NIGHT } from '../theme'
import { COUNT, HOOK, scenesOf } from '../timeline'
import type { ReelFinalVoteDto, ReelProps, ReelResponse, ScrutinDetailDto } from '../types'

const MOST_FOLLOWED_CRITERION = 'Critère : le scrutin qui compte le plus de votants (pour, contre, abstention).'

/** Rang d'affichage des positions : les groupes sont rangés par position (RM-11). */
const POSITION_ORDER: (LeadingPosition | null)[] = ['for', 'against', 'abstention', null]
const POSITION_LABELS: Record<LeadingPosition, string> = { for: 'Pour', against: 'Contre', abstention: 'Abstention' }

const CLAMP = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const
const DISPLAY: CSSProperties = { fontFamily: FONT_DISPLAY, fontWeight: 800 }

function detailOf(details: ReelProps['details'], uid: string): ScrutinDetailDto {
  const detail = details[uid]
  if (!detail) throw new Error(`Détail du scrutin ${uid} absent des propriétés`)
  return detail
}

const tallySize = (t: ScrutinDetailDto['groups'][number]['tally']) => t.votes_for + t.votes_against + t.abstentions + t.not_voting

/** Groupes qui ont pris part au vote, rangés par position, dans l'ordre publié à position égale. */
function groupsByPosition(detail: ScrutinDetailDto) {
  return POSITION_ORDER.map((position) => ({
    position,
    groups: detail.groups.filter((g) => tallySize(g.tally) > 0 && leadingPosition(g.tally) === position),
  })).filter(({ groups }) => groups.length > 0)
}

function plural(count: number, one: string, many: string) {
  return count > 1 ? many : one
}

function dayLabel(date: string) {
  const day = new Date(`${date}T12:00:00`)
  const weekday = day.toLocaleDateString('fr-FR', { weekday: 'short' }).replace('.', '')
  return `${capitalize(weekday)}. ${day.getDate()}`
}

function clampLines(lines: number): CSSProperties {
  return { display: '-webkit-box', WebkitLineClamp: lines, WebkitBoxOrient: 'vertical', overflow: 'hidden' }
}

/** Les trois décomptes d'un scrutin, dans l'ordre pour, contre, abstention ; `short` pour une ligne serrée. */
function tallyItems(t: { votes_for: number; votes_against: number; abstentions: number }) {
  return [
    { key: 'for', value: t.votes_for, label: 'pour', short: 'pour' },
    { key: 'against', value: t.votes_against, label: 'contre', short: 'contre' },
    { key: 'abstention', value: t.abstentions, label: plural(t.abstentions, 'abstention', 'abstentions'), short: 'abst.' },
  ]
}

/** Barre de répartition pour, contre, abstention ; `grow` de 0 à 1 l'allonge. */
function ShareBar({ items, height, grow = 1 }: { items: { key: string; value: number }[]; height: number; grow?: number }) {
  const total = items.reduce((sum, item) => sum + item.value, 0) || 1
  return (
    <div style={{ display: 'flex', gap: 6, height, borderRadius: height / 2, overflow: 'hidden', flexShrink: 0 }}>
      {items.filter((item) => item.value > 0).map((item) => (
        <div key={item.key} style={{ width: `${(item.value / total) * 100 * grow}%`, background: POSITION_COLORS[item.key] }} />
      ))}
    </div>
  )
}

/* ------------------------------------------------------------------ accroche */

const HOOK_KICKER: Record<ReelResponse['kind'], string> = {
  veille: 'Le vote du jour',
  semaine: 'Le vote de la semaine',
  lundi: 'Le vote de la période',
}

/**
 * Le scrutin le plus suivi, posé en question : son intitulé, sans son sort, que
 * la scène suivante donne avec le décompte. Titre présent dès la première image,
 * qui sert de couverture et que la boucle retrouve.
 */
function Hook({ reel, detail }: { reel: ReelResponse; detail: ScrutinDetailDto }) {
  const { post } = useLayout()
  const mf = reel.most_followed!
  const label = detail.dossier_label
  const subject = useEnter(8, 30)
  const question = useEnter(16, 40)
  return (
    <Stage>
      <div style={{ marginTop: post ? 36 : 56, display: 'flex' }}>
        <Kicker>{HOOK_KICKER[reel.kind]}</Kicker>
      </div>
      {label ? (
        <>
          <div style={{ ...DISPLAY, marginTop: 28, fontSize: label.length > 60 ? 80 : 104, lineHeight: 0.98, letterSpacing: -2.5, ...clampLines(4) }}>{label}</div>
          <div style={{ ...subject, marginTop: 16, fontSize: 32, fontWeight: 500, lineHeight: 1.25, color: NIGHT.inkSoft, ...clampLines(1) }}>{capitalize(mf.subject)}</div>
        </>
      ) : (
        <div style={{ ...DISPLAY, marginTop: 28, fontSize: 64, lineHeight: 1.06, letterSpacing: -1.5, ...clampLines(5) }}>{capitalize(mf.subject)}</div>
      )}
      <div style={{ ...question, ...DISPLAY, marginTop: 28, fontSize: 58, fontWeight: 700, lineHeight: 1.12, letterSpacing: -1 }}>
        Qui a voté <span style={{ color: NIGHT.for }}>pour</span>&nbsp;?
        <br />
        Qui a voté <span style={{ color: NIGHT.against }}>contre</span>&nbsp;?
      </div>
    </Stage>
  )
}

/* ---------------------------------------------------- hémicycle et décompte */

/** Hauteurs de la scène du décompte autour de l'hémicycle : titre au-dessus, barre, tuiles et sort au-dessous. */
const COUNT_HEAD = 136
const COUNT_FOOT = 475

/** Place de l'hémicycle dans la scène du décompte : centré entre le titre et la barre, aussi large que possible. */
function useCountRect() {
  const { zone, width, stage } = useLayout()
  const free = stage.height - COUNT_HEAD - COUNT_FOOT
  const size = Math.min(width, (free - 40) / PLAN_RATIO)
  return { left: zone.left + (width - size) / 2, top: stage.top + COUNT_HEAD + (free - size * PLAN_RATIO) / 2, width: size }
}

/**
 * Hémicycle du scrutin le plus suivi, un seul d'une scène à l'autre : allumé aux
 * couleurs des groupes sous l'accroche, il monte à sa place du décompte et
 * chaque siège y prend la couleur de sa position.
 */
function OpeningHemicycle({ detail }: { detail: ScrutinDetailDto }) {
  const frame = useCurrentFrame()
  const { hero, post } = useLayout()
  const count = useCountRect()
  const move = interpolate(frame, [HOOK, HOOK + 20], [0, 1], { ...CLAMP, easing: Easing.inOut(Easing.cubic) })
  const out = post ? 0 : interpolate(frame, [HOOK + COUNT - FADE, HOOK + COUNT], [0, 1], { ...CLAMP, easing: Easing.in(Easing.cubic) })
  const at = (from: number, to: number) => from + (to - from) * move
  return (
    <div
      style={{
        position: 'absolute',
        left: at(hero.left, count.left),
        top: at(hero.top, count.top),
        width: at(hero.width, count.width),
        opacity: 1 - out,
        transform: `translateY(${-out * 44}px)`,
      }}
    >
      <Hemicycle detail={detail} light={-200} flip={HOOK + 14} />
    </div>
  )
}

/** Décompte de toute l'Assemblée sous l'hémicycle, puis le sort publié. */
function CountScene({ reel, detail }: { reel: ReelResponse; detail: ScrutinDetailDto }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const mf = reel.most_followed!
  const items = tallyItems(mf.tally)
  const title = useEnter(0, 30)
  const grow = interpolate(frame, [14, 44], [0, 1], { ...CLAMP, easing: Easing.out(Easing.cubic) })
  const tiles = useEnter(12, 40)
  const band = spring({ frame: frame - 58, fps, config: { damping: 12, stiffness: 160 } })
  const tone = outcomeTone(detail.outcome_code)
  const word = capitalize(detail.outcome_code)
  return (
    <Stage>
      <div style={{ ...title, flexShrink: 0, height: COUNT_HEAD - 34, marginTop: 34 }}>
        <div style={{ ...DISPLAY, fontSize: 46, fontWeight: 700, lineHeight: 1.1, letterSpacing: -0.5, ...clampLines(2) }}>{detail.dossier_label ?? capitalize(mf.subject)}</div>
      </div>
      <div style={{ flex: 1 }} />
      <div style={{ opacity: tiles.opacity }}>
        <ShareBar items={items} height={22} grow={grow} />
      </div>
      <div style={{ ...tiles, marginTop: 22, display: 'flex', gap: 18, flexShrink: 0 }}>
        {items.map((item) => (
          <div key={item.key} style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 6, borderRadius: 28, background: POSITION_SOFT[item.key], padding: '26px 0 26px 28px' }}>
            <span style={{ ...DISPLAY, fontSize: 124, lineHeight: 0.92, letterSpacing: -4, color: POSITION_COLORS[item.key], fontVariantNumeric: 'tabular-nums' }}>
              <BigCount value={item.value} start={16} />
            </span>
            <span style={{ fontSize: 36, fontWeight: 700, lineHeight: 1.2 }}>{item.label}</span>
          </div>
        ))}
      </div>
      <div
        style={{
          marginTop: 22,
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 28,
          borderRadius: 32,
          background: POSITION_COLORS[tone],
          color: NIGHT.backgroundDeep,
          padding: '22px 36px 26px',
          opacity: Math.min(1, band * 2),
          transform: `scale(${0.7 + 0.3 * band})`,
        }}
      >
        {tone !== 'abstention' && (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: 104, height: 104, borderRadius: 52, background: NIGHT.backgroundDeep, flexShrink: 0 }}>
            <OutcomeIcon code={detail.outcome_code} size={64} color={POSITION_COLORS[tone]} />
          </div>
        )}
        <div style={{ display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <span style={{ ...DISPLAY, fontSize: word.length > 8 ? 76 : 108, lineHeight: 1, letterSpacing: -2, textTransform: 'uppercase' }}>{word}</span>
          <span style={{ fontSize: 30, fontWeight: 700, lineHeight: 1.25 }}>{capitalize(mf.outcome_label)}</span>
        </div>
      </div>
    </Stage>
  )
}

/* -------------------------------------------------------------------- groupes */

/** Chaque groupe sous sa position la plus fréquente, avec le nombre de ses députés qui l'ont prise (RM-03, RM-11). */
function GroupsScene({ reel, detail }: { reel: ReelResponse; detail: ScrutinDetailDto }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const { stage } = useLayout()
  const mf = reel.most_followed!
  const head = useEnter(0, 30)
  const panels = groupsByPosition(detail)
  const totals: Record<LeadingPosition, number> = { for: mf.tally.votes_for, against: mf.tally.votes_against, abstention: mf.tally.abstentions }
  const rowsOf = (count: number) => Math.ceil(count / 2)
  const rows = panels.reduce((sum, panel) => sum + rowsOf(panel.groups.length), 0)
  // Tout doit tenir : au-delà de la hauteur libre, lignes et titres rétrécissent ensemble.
  const needed = panels.length * 132 + rows * 77 + (rows - panels.length) * 12 + (panels.length - 1) * 18
  const unit = Math.min(1, (stage.height - 260) / needed)
  return (
    <Stage>
      <div style={{ ...head, marginTop: 34, flexShrink: 0 }}>
        <div style={{ ...DISPLAY, fontSize: 54, lineHeight: 1.05, letterSpacing: -1 }}>Groupe par groupe</div>
        <div style={{ marginTop: 6, fontSize: 30, fontWeight: 500, lineHeight: 1.3, color: NIGHT.inkSoft }}>Position la plus fréquente dans chaque groupe</div>
      </div>
      <div style={{ marginTop: 24, flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', gap: 18 }}>
        {panels.map(({ position, groups }, index) => {
          const enter = spring({ frame: frame - 6 - index * 9, fps, config: { damping: 200 } })
          const tone = position ? POSITION_COLORS[position] : NIGHT.inkSoft
          const chipsFrom = 12 + index * 9
          return (
            <section
              key={position ?? 'none'}
              style={{
                flex: `${rowsOf(groups.length) + 1.3} 1 0`,
                minHeight: 0,
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'center',
                gap: 16 * unit,
                borderRadius: 32,
                background: position ? POSITION_SOFT[position] : NIGHT.panel,
                padding: `${18 * unit}px 28px ${24 * unit}px`,
                opacity: enter,
                transform: `translateY(${(1 - enter) * 50}px)`,
              }}
            >
              <div style={{ ...DISPLAY, display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', fontSize: 60 * unit, lineHeight: 1.05, letterSpacing: -1, color: tone }}>
                <span>{position ? POSITION_LABELS[position] : 'Sans majorité'}</span>
                {position && (
                  <span style={{ fontVariantNumeric: 'tabular-nums' }}>
                    <BigCount value={totals[position]} start={chipsFrom} duration={20} />
                  </span>
                )}
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', columnGap: 14, rowGap: 12 * unit }}>
                {groups.map((g, i) => {
                  const pop = spring({ frame: frame - chipsFrom - i * 3, fps, config: { damping: 14, stiffness: 170 } })
                  const count = position ? { for: g.tally.votes_for, against: g.tally.votes_against, abstention: g.tally.abstentions }[position] : null
                  return (
                    <div
                      key={g.group_uid ?? g.abbrev}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 16,
                        borderRadius: 999,
                        background: NIGHT.chip,
                        padding: `${9 * unit}px 26px ${11 * unit}px 16px`,
                        opacity: Math.min(1, pop * 2),
                        transform: `scale(${0.8 + 0.2 * pop})`,
                      }}
                    >
                      <span style={{ width: 36 * unit, height: 36 * unit, borderRadius: 18, background: g.color ?? NIGHT.notVoting, border: '3px solid rgba(255, 255, 255, 0.85)', flexShrink: 0 }} />
                      <span style={{ fontSize: 46 * unit, fontWeight: 800, lineHeight: 1.2, whiteSpace: 'nowrap' }}>{g.abbrev}</span>
                      {count !== null && <span style={{ marginLeft: 'auto', fontSize: 40 * unit, fontWeight: 600, lineHeight: 1.2, color: NIGHT.inkSoft, fontVariantNumeric: 'tabular-nums' }}>{count}</span>}
                    </div>
                  )
                })}
              </div>
            </section>
          )
        })}
      </div>
      <div style={{ ...head, flexShrink: 0, marginTop: 20, fontSize: 26, lineHeight: 1.35, color: NIGHT.inkSoft }}>{MOST_FOLLOWED_CRITERION}</div>
    </Stage>
  )
}

/* ---------------------------------------------------------------- période */

const PERIOD_KICKER: Record<ReelResponse['kind'], string> = {
  veille: 'Toute la journée',
  semaine: 'Toute la semaine',
  lundi: 'Sur la période',
}

/** Scrutins publics de la période dessinés un par un, au plus ; au-delà, une barre. */
const SQUARES_MAX = 60

/** Côté et écart des carrés pour que tous tiennent dans `width` sur au plus `maxHeight` de haut. */
function squareGrid(count: number, width: number, maxHeight: number) {
  for (let columns = 5; columns <= 30; columns++) {
    const gap = columns > 10 ? 10 : 18
    const side = Math.floor((width - (columns - 1) * gap) / columns)
    const rows = Math.ceil(count / columns)
    if (rows * side + (rows - 1) * gap <= maxHeight) return { side, gap }
  }
  return { side: 20, gap: 8 }
}

/** Nombres publiés de la période : scrutins publics, adoptés, rejetés, autres sorts à part (RM-03). */
function PeriodScene({ reel }: { reel: ReelResponse }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const { post, width } = useLayout()
  const totals = reel.totals ?? { votes: 0, adopted: 0, rejected: 0, others: [] }
  const week = reel.days.length > 1
  const head = useEnter(0, 40)
  const big = useEnter(6, 80)
  const split = useEnter(20, 60)
  const outcomes = [
    ...Array.from({ length: totals.adopted }, () => 'adopté'),
    ...Array.from({ length: totals.rejected }, () => 'rejeté'),
    ...Array.from({ length: Math.max(0, totals.votes - totals.adopted - totals.rejected) }, () => 'autre'),
  ]
  const grid = squareGrid(outcomes.length, width, week ? 170 : 250)
  const grow = interpolate(frame, [24, 50], [0, 1], CLAMP)
  const notes = [
    totals.others.length > 0 ? `Dont ${totals.others.map((other) => `${other.count} ${other.label}`).join(', ')}.` : null,
    reel.empty_week_note,
  ].filter((note): note is string => Boolean(note))
  const numberSize = (week ? 320 : 470) * (post && week ? 0.8 : 1)
  return (
    <Stage>
      <div style={{ ...head, marginTop: post ? 36 : 56, display: 'flex', flexDirection: 'column', gap: 24, flexShrink: 0 }}>
        <Kicker>{PERIOD_KICKER[reel.kind]}</Kicker>
        <span style={{ ...DISPLAY, fontSize: reel.period.label.length > 22 ? 60 : 86, lineHeight: 1, letterSpacing: -2 }}>{capitalize(reel.period.label)}</span>
      </div>
      <div style={{ ...big, flex: 1, minHeight: 0, display: 'flex', alignItems: 'center', gap: 36 }}>
        <span style={{ ...DISPLAY, fontSize: numberSize, lineHeight: 0.8, letterSpacing: -numberSize * 0.034 }}>
          <BigCount value={totals.votes} start={8} duration={24} />
        </span>
        <span style={{ ...DISPLAY, fontSize: week ? 68 : 84, fontWeight: 700, lineHeight: 0.98, letterSpacing: -1.5, color: NIGHT.inkSoft }}>
          {plural(totals.votes, 'vote', 'votes')}
          <br />
          {plural(totals.votes, 'public', 'publics')}
        </span>
      </div>
      {week && (
        <div style={{ ...split, display: 'flex', gap: 14, marginBottom: 22, flexShrink: 0 }}>
          {reel.days.map((day) => (
            <div key={day.date} style={{ flex: 1, borderRadius: 20, padding: '10px 0 12px', textAlign: 'center', background: NIGHT.panel }}>
              <div style={{ fontSize: 28, color: NIGHT.inkSoft }}>{dayLabel(day.date)}</div>
              <div style={{ ...DISPLAY, fontSize: 52, lineHeight: 1.1 }}>{day.votes}</div>
            </div>
          ))}
        </div>
      )}
      {outcomes.length > 0 && outcomes.length <= SQUARES_MAX ? (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: grid.gap, flexShrink: 0 }}>
          {outcomes.map((code, index) => {
            const pop = spring({ frame: frame - 22 - index * (outcomes.length > 12 ? 1 : 3), fps, config: { damping: 13, stiffness: 180 } })
            return (
              <div
                key={index}
                style={{
                  width: grid.side,
                  height: grid.side,
                  borderRadius: grid.side * 0.24,
                  background: POSITION_COLORS[outcomeTone(code)],
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  transform: `scale(${pop})`,
                }}
              >
                <OutcomeIcon code={code} size={grid.side * 0.56} color={NIGHT.backgroundDeep} />
              </div>
            )
          })}
        </div>
      ) : (
        totals.votes > 0 && (
          <ShareBar
            items={[
              { key: 'for', value: totals.adopted },
              { key: 'against', value: totals.rejected },
              { key: 'abstention', value: Math.max(0, totals.votes - totals.adopted - totals.rejected) },
            ]}
            height={44}
            grow={grow}
          />
        )
      )}
      <div style={{ ...split, marginTop: 22, display: 'flex', gap: 18, flexShrink: 0 }}>
        {[
          { key: 'for', value: totals.adopted, label: plural(totals.adopted, 'adopté', 'adoptés') },
          { key: 'against', value: totals.rejected, label: plural(totals.rejected, 'rejeté', 'rejetés') },
        ].map((item) => (
          <div key={item.key} style={{ flex: 1, display: 'flex', alignItems: 'baseline', gap: 18, borderRadius: 28, background: POSITION_SOFT[item.key], padding: '18px 30px 22px' }}>
            <span style={{ ...DISPLAY, fontSize: 112, lineHeight: 1, letterSpacing: -3, color: POSITION_COLORS[item.key] }}>
              <BigCount value={item.value} start={22} duration={20} />
            </span>
            <span style={{ fontSize: 44, fontWeight: 700, lineHeight: 1.2 }}>{item.label}</span>
          </div>
        ))}
      </div>
      {notes.map((note) => (
        <span key={note} style={{ ...split, marginTop: 16, fontSize: 30, lineHeight: 1.3, color: NIGHT.inkSoft, flexShrink: 0 }}>
          {note}
        </span>
      ))}
    </Stage>
  )
}

/* ------------------------------------------------------- votes sur l'ensemble */

function OutcomePill({ code, size }: { code: string; size: number }) {
  return (
    <span
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: size * 0.28,
        borderRadius: 999,
        background: POSITION_COLORS[outcomeTone(code)],
        color: NIGHT.backgroundDeep,
        padding: `${size * 0.2}px ${size * 0.6}px ${size * 0.24}px ${size * 0.45}px`,
        fontSize: size,
        fontWeight: 800,
        lineHeight: 1.2,
        whiteSpace: 'nowrap',
      }}
    >
      <OutcomeIcon code={code} size={size * 0.9} color={NIGHT.backgroundDeep} />
      {capitalize(code)}
    </span>
  )
}

/**
 * Un vote sur l'ensemble : sort, intitulé officiel, décompte de toute
 * l'Assemblée publié dans la synthèse officielle, groupes rangés par position
 * (RM-03, RM-11). `unit` rétrécit la carte quand la place manque, `lines`
 * borne son intitulé.
 */
function FinalVote({ vote, detail, unit, lines, start }: { vote: ReelFinalVoteDto; detail: ScrutinDetailDto; unit: number; lines: number; start: number }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const enter = spring({ frame: frame - start, fps, config: { damping: 200 } })
  const items = tallyItems(detail.synthesis.tally)
  const grow = interpolate(frame, [start + 6, start + 30], [0, 1], { ...CLAMP, easing: Easing.out(Easing.cubic) })
  return (
    <div
      style={{
        flex: '1 1 0',
        minHeight: 0,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        gap: 14 * unit,
        borderRadius: 32,
        background: NIGHT.panel,
        padding: `${22 * unit}px 32px ${26 * unit}px`,
        opacity: enter,
        transform: `translateY(${(1 - enter) * 50}px)`,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <OutcomePill code={vote.outcome_code} size={36 * unit} />
        <span style={{ fontSize: 30 * unit, fontWeight: 600, color: NIGHT.inkSoft }}>n° {vote.number}</span>
      </div>
      <div style={{ ...DISPLAY, flexShrink: 0, fontSize: 42 * unit, fontWeight: 700, lineHeight: 1.14, letterSpacing: -0.5, ...clampLines(lines) }}>{capitalize(vote.text_label)}</div>
      <ShareBar items={items} height={16} grow={grow} />
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
        {items.map((item) => (
          <span key={item.key} style={{ ...DISPLAY, fontSize: 58 * unit, lineHeight: 1, letterSpacing: -1.5, color: POSITION_COLORS[item.key], fontVariantNumeric: 'tabular-nums', whiteSpace: 'nowrap' }}>
            <BigCount value={item.value} start={start + 6} duration={20} />
            <span style={{ fontSize: 30 * unit, fontWeight: 700, letterSpacing: 0, color: NIGHT.ink }}> {item.short}</span>
          </span>
        ))}
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4, paddingTop: 14 * unit, borderTop: `2px solid ${NIGHT.line}` }}>
        {groupsByPosition(detail).map(({ position, groups }) => (
          <div key={position ?? 'none'} style={{ fontSize: 30 * unit, fontWeight: 600, lineHeight: 1.3 }}>
            <span style={{ fontWeight: 800, color: position ? POSITION_COLORS[position] : NIGHT.inkSoft }}>{position ? POSITION_LABELS[position] : 'Sans majorité'}</span>
            &nbsp; {groups.map((g) => g.abbrev).join(' · ')}
          </div>
        ))}
      </div>
    </div>
  )
}

/** Carte des votes sur l'ensemble : deux par carte, davantage quand ils sont trop nombreux (T7). Aucun n'est retiré. */
function FinalVotes({ votes, details, count }: { votes: ReelFinalVoteDto[]; details: ReelProps['details']; count: number }) {
  const head = useEnter(0, 30)
  const { post } = useLayout()
  // Deux cartes tiennent à pleine taille dans le Reel ; le post, moins haut, et les cartes plus nombreuses rétrécissent.
  const unit = (votes.length > 2 ? Math.max(0.5, 2.1 / votes.length) : 1) * (post ? 0.9 : 1)
  return (
    <Stage>
      <div style={{ ...head, ...DISPLAY, marginTop: 34, fontSize: 50, lineHeight: 1.05, letterSpacing: -1, flexShrink: 0 }}>
        {plural(count, 'Vote sur l’ensemble d’un texte', 'Votes sur l’ensemble d’un texte')}
      </div>
      <div style={{ marginTop: 24, flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', gap: 18 }}>
        {votes.map((vote, index) => (
          <FinalVote key={vote.scrutin_uid} vote={vote} detail={detailOf(details, vote.scrutin_uid)} unit={unit} lines={votes.length > 2 ? 2 : 3} start={4 + index * 8} />
        ))}
      </div>
    </Stage>
  )
}

/* ------------------------------------------------------------- composition */

/** Plan des sièges, pâle, sous la zone de contenu : il occupe le bas de l'image, que la légende des réseaux recouvre. */
function Watermark() {
  const frame = useCurrentFrame()
  const { post, zone } = useLayout()
  const { height } = useVideoConfig()
  if (post) return null
  const enter = interpolate(frame, [0, 12], [0, 1], CLAMP)
  return (
    <div style={{ position: 'absolute', left: -160, top: height - zone.bottom - 10, width: 1400, opacity: enter * 0.1 }}>
      <Hemicycle detail={null} light={0} flip={null} />
    </div>
  )
}

/**
 * Reel de votes, de la veille ou de la semaine (RM-03, RM-04) : le scrutin le
 * plus suivi ouvre (intitulé, hémicycle, décompte et sort, groupes rangés par
 * position, critère), puis les nombres de la période et chaque vote sur
 * l'ensemble dans l'ordre de l'API, enfin l'adresse du site. Aucun vote n'est
 * retiré, aucun chiffre n'est calculé ici (RM-10) : nombres et décomptes sont
 * ceux de l'API.
 */
export function Votes(props: ReelProps) {
  useFonts()
  const { reel, details, site } = props
  const scenes = scenesOf(props)
  const mf = reel.most_followed
  const main = mf ? detailOf(details, mf.uid) : null
  const cardLabel = (card: number) => (scenes.cards.groups.length > 1 ? `${card + 1} / ${scenes.cards.groups.length}` : reel.period.label)

  const list: { label: string; node: ReactNode }[] = []
  if (mf && main) {
    list.push({ label: reel.period.label, node: <Hook reel={reel} detail={main} /> })
    list.push({ label: `Scrutin n° ${mf.number}`, node: <CountScene reel={reel} detail={main} /> })
    list.push({ label: `Scrutin n° ${mf.number}`, node: <GroupsScene reel={reel} detail={main} /> })
  }
  list.push({ label: 'Assemblée nationale', node: <PeriodScene reel={reel} /> })
  scenes.cards.groups.forEach((indexes, card) => {
    list.push({ label: cardLabel(card), node: <FinalVotes votes={indexes.map((i) => reel.final_votes[i])} details={details} count={reel.final_votes.length} /> })
  })
  list.push({ label: site, node: <BrandOutro site={site} note={reel.reconstructed_note} detail={main} bar={false} /> })

  const marks = scenes.lengths.map((_, index) => scenes.lengths.slice(0, index).reduce((sum, length) => sum + length, 0))
  // Le filigrane prend le relais de l'hémicycle du scrutin, qui s'efface à la fin du décompte.
  const watermarkFrom = main ? marks[2] : 0
  return (
    <Night>
      <Soundtrack audio={props.audio} />
      {main && (
        <Sequence durationInFrames={HOOK + COUNT}>
          <OpeningHemicycle detail={main} />
        </Sequence>
      )}
      <Sequence from={watermarkFrom} durationInFrames={marks[list.length - 1] - watermarkFrom}>
        <Watermark />
      </Sequence>
      {list.map(({ node }, index) => (
        <Sequence key={marks[index]} from={marks[index]} durationInFrames={scenes.lengths[index]}>
          <Fade length={scenes.lengths[index]} first={index === 0} last={index === list.length - 1}>
            {node}
          </Fade>
        </Sequence>
      ))}
      <TopBar marks={marks} total={scenes.total} labels={list.map(({ label }) => label)} />
    </Night>
  )
}
