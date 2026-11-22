import { Sequence } from 'remotion'
import { Cards, CountUp, Frame, Intro, Kicker, Note, Outro, Soundtrack, Title, useFade, VoteImage, VoteImages } from '../components'
import { COLORS } from '../theme'
import { INTRO, MOST_FOLLOWED_IMAGE, MOST_FOLLOWED_TITLE, NUMBERS, scenesOf } from '../timeline'
import type { OutcomeCountDto, ReelDayDto, ReelProps, ScrutinDetailDto } from '../types'

const MOST_FOLLOWED_CRITERION = 'Critère : le scrutin qui compte le plus de votants (pour, contre, abstention).'

function detailOf(details: ReelProps['details'], uid: string): ScrutinDetailDto {
  const detail = details[uid]
  if (!detail) throw new Error(`Détail du scrutin ${uid} absent des propriétés`)
  return detail
}

function dayLabel(date: string) {
  const day = new Date(`${date}T12:00:00`)
  const weekday = day.toLocaleDateString('fr-FR', { weekday: 'short' }).replace('.', '')
  return `${weekday.charAt(0).toUpperCase()}${weekday.slice(1)}. ${day.getDate()}`
}

function Numbers({ totals, days, emptyNote }: { totals: OutcomeCountDto; days: ReelDayDto[]; emptyNote: string | null }) {
  const style = useFade(NUMBERS)
  return (
    <Frame>
      <div style={{ ...style, display: 'flex', flexDirection: 'column', gap: 56 }}>
        <CountUp value={totals.votes} label={totals.votes > 1 ? 'votes publics' : 'vote public'} color={COLORS.ink} length={NUMBERS} />
        <div style={{ display: 'flex', gap: 72 }}>
          <CountUp value={totals.adopted} label={totals.adopted > 1 ? 'adoptés' : 'adopté'} color={COLORS.adopted} length={NUMBERS} />
          <CountUp value={totals.rejected} label={totals.rejected > 1 ? 'rejetés' : 'rejeté'} color={COLORS.rejected} length={NUMBERS} />
        </div>
        {totals.others.length > 0 && (
          <Note>Dont {totals.others.map((other) => `${other.count} ${other.label}`).join(', ')}</Note>
        )}
        {days.length > 1 && (
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 16 }}>
            {days.map((day) => (
              <div
                key={day.date}
                style={{ border: `2px solid ${COLORS.line}`, borderRadius: 24, padding: '14px 22px', textAlign: 'center', minWidth: 132 }}
              >
                <div style={{ fontSize: 28, color: COLORS.inkSoft }}>{dayLabel(day.date)}</div>
                <div style={{ fontSize: 44, fontWeight: 700 }}>{day.votes}</div>
              </div>
            ))}
          </div>
        )}
        {emptyNote && <Note>{emptyNote}</Note>}
      </div>
    </Frame>
  )
}

function MostFollowedTitle({ subject }: { subject: string }) {
  const style = useFade(MOST_FOLLOWED_TITLE)
  return (
    <Frame>
      <div style={{ ...style, display: 'flex', flexDirection: 'column', gap: 32 }}>
        <Kicker>Le scrutin le plus suivi</Kicker>
        <Title size={64}>{subject}</Title>
        <Note>{MOST_FOLLOWED_CRITERION}</Note>
      </div>
    </Frame>
  )
}

/**
 * Reel de votes, de la veille ou de la semaine (RM-03, RM-04) : nombres de la
 * période, chaque vote sur l'ensemble dans l'ordre de l'API, puis le scrutin le
 * plus suivi. Aucun vote n'est retiré, aucun chiffre n'est calculé ici (RM-10).
 */
export function Votes(props: ReelProps) {
  const { reel, details, site } = props
  const scenes = scenesOf(props)
  const totals = reel.totals ?? { votes: 0, adopted: 0, rejected: 0, others: [] }
  const cardsFrom = INTRO + scenes.numbers
  const mostFollowedFrom = cardsFrom + scenes.cards.groups.length * scenes.cards.each
  const outroFrom = mostFollowedFrom + scenes.mostFollowed
  const kicker = reel.kind === 'semaine' ? 'La semaine à l’Assemblée nationale' : 'Les votes de l’Assemblée nationale'

  return (
    <>
      <Soundtrack audio={props.audio} />
      <Sequence durationInFrames={INTRO}>
        <Intro kicker={kicker} title={reel.period.label} />
      </Sequence>
      <Sequence from={INTRO} durationInFrames={NUMBERS}>
        <Numbers totals={totals} days={reel.days} emptyNote={reel.empty_week_note} />
      </Sequence>
      <Cards
        from={cardsFrom}
        plan={scenes.cards}
        render={(indexes) => (
          <VoteImages details={indexes.map((i) => detailOf(details, reel.final_votes[i].scrutin_uid))} site={site} />
        )}
      />
      {reel.most_followed && (
        <>
          <Sequence from={mostFollowedFrom} durationInFrames={MOST_FOLLOWED_TITLE}>
            <MostFollowedTitle subject={reel.most_followed.subject} />
          </Sequence>
          <Sequence from={mostFollowedFrom + MOST_FOLLOWED_TITLE} durationInFrames={MOST_FOLLOWED_IMAGE}>
            <VoteImage detail={detailOf(details, reel.most_followed.uid)} site={site} />
          </Sequence>
        </>
      )}
      <Sequence from={outroFrom}>
        <Outro site={site} path={reel.link_path} notes={[reel.reconstructed_note]} />
      </Sequence>
    </>
  )
}
