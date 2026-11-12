-- Comptes rendus de séance: version lue, rattachements aux scrutins et
-- explications de vote (SPEC-explications-vote, ADR-003).
-- Une séance remplacée emporte ses rattachements et ses explications (RM-08).

CREATE TABLE sitting_records (
    uid                  TEXT PRIMARY KEY,
    sitting_ref          TEXT NOT NULL,
    sitting_date         DATE NOT NULL,
    version              TEXT NOT NULL,
    generated_at         TIMESTAMPTZ NOT NULL,
    keys_fingerprint     TEXT NOT NULL,
    results_attached     INTEGER NOT NULL,
    results_unmatched    INTEGER NOT NULL,
    results_ambiguous    INTEGER NOT NULL,
    blocks_unattached    INTEGER NOT NULL,
    headings_unrecognized INTEGER NOT NULL
);

CREATE INDEX sitting_records_sitting_ref_idx ON sitting_records (sitting_ref);

CREATE TABLE sitting_record_scrutins (
    sitting_record_uid TEXT NOT NULL REFERENCES sitting_records (uid) ON DELETE CASCADE,
    scrutin_uid        TEXT NOT NULL,
    has_block          BOOLEAN NOT NULL,
    PRIMARY KEY (sitting_record_uid, scrutin_uid)
);

CREATE INDEX sitting_record_scrutins_scrutin_idx ON sitting_record_scrutins (scrutin_uid);

CREATE TABLE vote_explanations (
    sitting_record_uid TEXT NOT NULL REFERENCES sitting_records (uid) ON DELETE CASCADE,
    scrutin_uid        TEXT NOT NULL,
    rank               INTEGER NOT NULL,
    speaker_uid        TEXT NOT NULL,
    speaker_name       TEXT,
    group_basis        TEXT NOT NULL CHECK (group_basis IN ('scrutin', 'membership', 'undetermined')),
    group_uid          TEXT,
    anchor             TEXT NOT NULL,
    heading            TEXT NOT NULL,
    text               TEXT NOT NULL,
    PRIMARY KEY (sitting_record_uid, scrutin_uid, rank),
    CHECK ((group_basis = 'undetermined') = (group_uid IS NULL))
);

CREATE INDEX vote_explanations_scrutin_idx ON vote_explanations (scrutin_uid);
