-- Historique des extraits de programme ajoutés ou retirés par une publication
-- (le seed des candidatures). « Publié » = enregistré par le seed en production.
CREATE TABLE candidate_proposal_changes (
    id bigserial PRIMARY KEY,
    candidate_id text NOT NULL,
    family_code text NOT NULL,
    excerpt text NOT NULL,
    change text NOT NULL CHECK (change IN ('added', 'removed')),
    published_on date NOT NULL
);

CREATE INDEX candidate_proposal_changes_published_on_idx
    ON candidate_proposal_changes (published_on);

-- Compare la table temporaire `candidate_proposals_before` (photographie prise
-- par le seed avant le remplacement) aux extraits actuels sur
-- (candidate_id, family_code, excerpt), sans source_url : un extrait réécrit à
-- l'identique n'est pas un changement. Une candidature absente de la
-- photographie voit tous ses extraits ajoutés.
CREATE FUNCTION record_candidate_proposal_changes() RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO candidate_proposal_changes (candidate_id, family_code, excerpt, change, published_on)
    SELECT DISTINCT current.candidate_id, current.family_code, current.excerpt, 'added', CURRENT_DATE
      FROM candidate_program_proposals current
     WHERE NOT EXISTS (
         SELECT 1 FROM candidate_proposals_before before
          WHERE before.candidate_id = current.candidate_id
            AND before.family_code = current.family_code
            AND before.excerpt = current.excerpt
     );

    INSERT INTO candidate_proposal_changes (candidate_id, family_code, excerpt, change, published_on)
    SELECT DISTINCT before.candidate_id, before.family_code, before.excerpt, 'removed', CURRENT_DATE
      FROM candidate_proposals_before before
     WHERE NOT EXISTS (
         SELECT 1 FROM candidate_program_proposals current
          WHERE current.candidate_id = before.candidate_id
            AND current.family_code = before.family_code
            AND current.excerpt = before.excerpt
     );
END;
$$;
