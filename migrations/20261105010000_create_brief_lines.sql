-- Lignes « En bref », échecs du modèle par version et avancement de la passe
-- par dossier (SPEC-en-bref RM-05, RM-13, RM-16, RM-17, CU-02).

-- Une ligne appartient à l'empreinte de son article, pas à un dossier ni à une
-- version (RM-05): un article identique dans deux textes reprend la même ligne.
-- 'ready': ligne publiable, jamais réécrite. 'rejected': refus avec son motif
-- (RM-13), renvoyé au modèle seulement sous une autre version du prompt
-- (RM-17). Le modèle vaut 'rule' quand le code a écrit la ligne (RM-06).
-- cited_sources: articles cités transmis au modèle avec leur date de rédaction
-- (RM-16), en liste de textes.
CREATE TABLE brief_lines (
    fingerprint TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    line TEXT,
    rejection TEXT,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    cited_sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    CONSTRAINT brief_lines_status CHECK (status IN ('ready', 'rejected')),
    CONSTRAINT brief_lines_content CHECK (
        (status = 'ready' AND line IS NOT NULL AND rejection IS NULL)
        OR (status = 'rejected' AND line IS NULL AND rejection IS NOT NULL)
    )
);

-- Passes consécutives où le modèle a échoué pour une version. Au troisième
-- échec, les articles envoyés sont refusés (CU-02, Erreurs); une passe qui
-- réussit sur tous ses paquets remet le compteur à zéro en supprimant la ligne.
CREATE TABLE text_version_brief_attempts (
    document_uid TEXT PRIMARY KEY REFERENCES text_versions(document_uid) ON DELETE CASCADE,
    failures SMALLINT NOT NULL,
    last_error TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT text_version_brief_attempts_failures CHECK (failures >= 1)
);

-- Avancement de la dernière passe sur un dossier: la version dont les articles
-- ont été classés, le prompt sous lequel, ce qui reste sans ligne (source
-- manquante RM-08, appel en échec) et ce qui est refusé. Il décide si un
-- dossier mérite un nouveau passage (RM-15) sans relire ses articles.
CREATE TABLE dossier_brief_progress (
    dossier_uid TEXT PRIMARY KEY,
    document_uid TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    pending_count INTEGER NOT NULL,
    rejected_count INTEGER NOT NULL,
    attempted_on DATE NOT NULL,
    CONSTRAINT dossier_brief_progress_counts CHECK (pending_count >= 0 AND rejected_count >= 0)
);
