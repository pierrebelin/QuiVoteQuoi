-- Versions d'un texte de loi et articles de celles dont le texte a été lu
-- (SPEC-en-bref RM-01, RM-03, RM-04, RM-14, CU-01).
--
-- Une version est un texte déposé ou adopté en séance, publié par l'Assemblée
-- (chamber = 'an') ou par le Sénat seulement (chamber = 'sn', enregistrée sans
-- texte). Aucune clé étrangère vers legislative_dossiers: l'archive nomme des
-- dossiers que le site n'ingère pas toujours, et une version ne doit pas être
-- refusée pour cela.
CREATE TABLE text_versions (
    document_uid TEXT PRIMARY KEY,
    dossier_uid TEXT NOT NULL,
    kind TEXT NOT NULL,
    chamber TEXT NOT NULL,
    -- Date de dépôt, ou de publication pour un texte adopté qui n'en a pas.
    deposited_on DATE NOT NULL,
    title TEXT NOT NULL,
    -- 'unread': texte pas encore lu. 'read': articles enregistrés. 'missing':
    -- texte introuvable ou sans aucun article reconnu, définitif (CU-01).
    reading_status TEXT NOT NULL DEFAULT 'unread',
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT text_versions_kind CHECK (kind IN ('deposited', 'adopted')),
    CONSTRAINT text_versions_chamber CHECK (chamber IN ('an', 'sn')),
    CONSTRAINT text_versions_reading_status
        CHECK (reading_status IN ('unread', 'read', 'missing'))
);

CREATE INDEX idx_text_versions_dossier ON text_versions(dossier_uid);

-- Articles du dispositif d'une version lue (RM-04), remplacés en bloc avec le
-- statut de lecture. rank part de 1. La division est le chemin des divisions
-- sous lesquelles l'article est rangé. La fingerprint (sha256:<hex>) est celle
-- du contenu normalisé, hors numéro (RM-03): deux articles de même empreinte
-- ont le même contenu.
CREATE TABLE text_version_articles (
    document_uid TEXT NOT NULL REFERENCES text_versions(document_uid) ON DELETE CASCADE,
    rank INTEGER NOT NULL,
    label TEXT NOT NULL,
    division TEXT,
    content TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    PRIMARY KEY (document_uid, rank),
    CONSTRAINT text_version_articles_rank CHECK (rank >= 1)
);

CREATE INDEX idx_text_version_articles_fingerprint ON text_version_articles(fingerprint);
