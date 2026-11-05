-- Cause d'absence de lien: l'auteur nomme par l'objet du scrutin n'est pas celui
-- de l'amendement portant ce numero (SPEC-sujet-amendements RM-11). Les liens deja stockes ne
-- changent qu'a la prochaine passe de rapprochement.
ALTER TABLE scrutin_amendment_citations
    DROP CONSTRAINT scrutin_amendment_citations_cause,
    ADD CONSTRAINT scrutin_amendment_citations_cause
        CHECK (cause IS NULL OR cause IN
            ('no_number', 'no_dossier', 'unresolved', 'ambiguous', 'author_mismatch'));
