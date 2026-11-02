-- Citations d'amendements relevees dans l'objet des scrutins (RM-06 / CU-01).
-- Projection recalculee a chaque passe: remplacee en bloc, sans etat propre.
-- Aucune cle etrangere: un scrutin ou un amendement peut manquer a la lecture
-- sans que la citation disparaisse (meme motif que scrutin_votes).
CREATE TABLE scrutin_amendment_citations (
    scrutin_uid TEXT NOT NULL,
    rank SMALLINT NOT NULL,
    cited_number TEXT,
    amendment_uid TEXT,
    cause TEXT,
    by_cited_article BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (scrutin_uid, rank),
    CONSTRAINT scrutin_amendment_citations_cause
        CHECK (cause IS NULL OR cause IN ('no_number', 'no_dossier', 'unresolved', 'ambiguous')),
    CONSTRAINT scrutin_amendment_citations_outcome
        CHECK ((amendment_uid IS NULL) = (cause IS NOT NULL)),
    CONSTRAINT scrutin_amendment_citations_method
        CHECK (amendment_uid IS NOT NULL OR NOT by_cited_article)
);

CREATE INDEX idx_scrutin_amendment_citations_amendment
    ON scrutin_amendment_citations(amendment_uid) WHERE amendment_uid IS NOT NULL;
