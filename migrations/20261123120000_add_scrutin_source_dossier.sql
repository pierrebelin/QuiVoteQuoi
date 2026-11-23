-- Dossier que le fichier du scrutin nommait, quand le rattachement retenu s'en
-- ecarte (dossier de l'acte de decision, ou du texte que l'objet nomme).
-- NULL quand le fichier et le rattachement concordent. Garde pour que l'ecart
-- avec la source reste visible a l'ecran (README.md §2).
ALTER TABLE scrutins
    ADD COLUMN source_dossier_uid TEXT,
    ADD COLUMN source_dossier_label TEXT,
    ADD CONSTRAINT scrutins_source_dossier_pair
        CHECK ((source_dossier_uid IS NULL) = (source_dossier_label IS NULL));
