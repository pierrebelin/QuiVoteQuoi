-- Scrutin public que la source rattache a une decision de seance
-- (`voteRefs` d'un acte `*-DEC`) : le vote qui a tranche une lecture, cite par
-- l'Assemblee elle-meme. Pas de cle etrangere : l'acte peut citer un scrutin
-- pas encore importe, la fiche se contente alors de ne pas l'afficher.
ALTER TABLE legislative_acts ADD COLUMN decision_scrutin_uid TEXT;
