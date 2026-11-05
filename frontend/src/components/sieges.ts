import type { GroupBreakdownDto, TallyDto, VoteDto } from '../types/scrutins'

/**
 * Couleur d'un siège, par position de vote.
 *
 * Les teintes de barre plutôt que celles de texte : cinq cent quatre-vingts
 * pastilles à pleine saturation se lisent comme un feu tricolore, donc comme un
 * jugement — le même motif que pour `VoteBar` (README.md §6).
 */
export const REMPLISSAGE_PAR_POSITION: Record<VoteDto['position'], string> = {
  for: 'fill-yes-bar',
  against: 'fill-no-bar',
  abstention: 'fill-abstain-bar',
  not_voting: 'fill-line-strong',
}

export const TEXTE_PAR_POSITION: Record<VoteDto['position'], string> = {
  for: 'text-yes',
  against: 'text-no',
  abstention: 'text-abstain',
  not_voting: 'text-ink-soft',
}

/**
 * Vote nominal augmenté de son groupe. Le groupe est porté par le bloc, pas par
 * le vote : sans cette reprise, l'infobulle d'un siège ne pourrait pas nommer le
 * groupe du député — et le déduire d'autre chose est précisément ce que
 * README.md §6 interdit.
 */
export type SiegeVote = VoteDto & {
  groupAbbrev: string | null
  groupLabel: string | null
}

export function siegesDesGroupes(groups: GroupBreakdownDto[]): SiegeVote[] {
  return groups.flatMap((groupe) =>
    groupe.votes.map((vote) => ({
      ...vote,
      groupAbbrev: groupe.abbrev,
      groupLabel: groupe.label,
    })),
  )
}

/**
 * Décompte reconstitué depuis les positions nominales.
 *
 * Sert à ce que la légende dise exactement ce que le dessin montre : deux totaux
 * différents sous un même graphique seraient une incohérence affichée.
 */
export function tallyDesVotes(votes: Pick<VoteDto, 'position'>[]): TallyDto {
  return {
    votes_for: votes.filter((v) => v.position === 'for').length,
    votes_against: votes.filter((v) => v.position === 'against').length,
    abstentions: votes.filter((v) => v.position === 'abstention').length,
    not_voting: votes.filter((v) => v.position === 'not_voting').length,
  }
}
