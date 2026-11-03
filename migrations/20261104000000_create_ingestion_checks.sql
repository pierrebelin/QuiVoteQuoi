-- Journal d'ingestion: une ligne par synchronisation tentee d'une source (RM-F1, RM-F2).
-- Les codes de source sont ceux de `IngestionSource::ALL`; un test les compare.
CREATE TABLE ingestion_checks (
    id BIGSERIAL PRIMARY KEY,
    source TEXT NOT NULL,
    attempted_at TIMESTAMPTZ NOT NULL,
    succeeded BOOLEAN NOT NULL,
    items_read BIGINT,
    failure_reason TEXT,
    CONSTRAINT ingestion_checks_source
        CHECK (source IN ('actor_registry', 'dossiers', 'scrutins', 'amendments')),
    CONSTRAINT ingestion_checks_outcome
        CHECK (
            (succeeded AND items_read IS NOT NULL AND failure_reason IS NULL)
            OR (NOT succeeded AND items_read IS NULL AND failure_reason IS NOT NULL)
        )
);

CREATE INDEX idx_ingestion_checks_source_attempted
    ON ingestion_checks(source, attempted_at DESC);
