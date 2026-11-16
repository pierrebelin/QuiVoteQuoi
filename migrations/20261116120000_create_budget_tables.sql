-- Budget de l'État : crédits de paiement votés en loi de finances initiale (LFI),
-- par mission et par programme (page /budget).
-- Un exercice est remplacé en bloc à chaque import : la suppression de la ligne
-- d'exercice emporte ses missions et ses programmes (ON DELETE CASCADE).
-- Le montant d'une mission n'est pas stocké : c'est la somme de ses programmes,
-- pour qu'il n'existe qu'une seule vérité.

CREATE TABLE budget_exercises (
    year                  SMALLINT PRIMARY KEY,
    stage                 TEXT NOT NULL CHECK (stage = 'lfi'),
    law_number            TEXT NOT NULL,
    law_date              DATE NOT NULL,
    credits_article       SMALLINT NOT NULL,
    adoption_procedure    TEXT NOT NULL CHECK (adoption_procedure IN ('vote', 'article_49_3')),
    legifrance_url        TEXT NOT NULL,
    source_title          TEXT NOT NULL,
    source_url            TEXT NOT NULL,
    source_published_on   DATE NOT NULL,
    source_downloaded_on  DATE NOT NULL,
    source_sha256         TEXT NOT NULL,
    imported_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE budget_missions (
    year          SMALLINT NOT NULL REFERENCES budget_exercises (year) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    account_kind  TEXT NOT NULL CHECK (account_kind IN ('budget_general', 'concours_financiers')),
    PRIMARY KEY (year, name)
);

CREATE TABLE budget_programmes (
    year          SMALLINT NOT NULL,
    number        SMALLINT NOT NULL,
    name          TEXT NOT NULL,
    mission_name  TEXT NOT NULL,
    cp_euros      BIGINT NOT NULL CHECK (cp_euros >= 0),
    PRIMARY KEY (year, number),
    FOREIGN KEY (year, mission_name) REFERENCES budget_missions (year, name) ON DELETE CASCADE
);

CREATE INDEX budget_programmes_mission_idx ON budget_programmes (year, mission_name);
