-- Budget de l'État : crédits de paiement de chaque action d'un programme, par
-- catégorie de dépense (nomenclature par nature, LOLF art. 5), tels que le
-- fichier « Crédits AE et CP votés » les donne, sous-actions fondues.
-- Le titre n'est pas stocké : c'est le premier chiffre de la catégorie.
-- La somme des lignes d'un programme est égale à ses CP (contrôle d'import) ;
-- une action votée à 0 € de CP reste listée. Remplacées avec l'exercice
-- (ON DELETE CASCADE depuis le programme).

CREATE TABLE budget_actions (
    year              SMALLINT NOT NULL,
    programme_number  SMALLINT NOT NULL,
    action_code       TEXT NOT NULL CHECK (btrim(action_code) <> ''),
    action_name       TEXT NOT NULL CHECK (btrim(action_name) <> ''),
    category          SMALLINT NOT NULL CHECK (category BETWEEN 10 AND 79),
    cp_euros          BIGINT NOT NULL CHECK (cp_euros >= 0),
    PRIMARY KEY (year, programme_number, action_code, category),
    FOREIGN KEY (year, programme_number)
        REFERENCES budget_programmes (year, number) ON DELETE CASCADE
);
