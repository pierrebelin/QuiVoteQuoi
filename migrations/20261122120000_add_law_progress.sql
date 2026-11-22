-- Où en est la loi (todo/navette/SPEC-navette.md, lot 1).
--
-- Le `statutConclusion` de chaque acte, tel que la source l'écrit : le sens
-- d'une décision (`*-DEC`) s'en déduit côté domaine.
ALTER TABLE legislative_acts ADD COLUMN conclusion_label TEXT;
ALTER TABLE legislative_acts ADD COLUMN conclusion_code TEXT;

-- Résumé de l'état calculé par le domaine (`LawProgress`), pour le filtre et
-- les compteurs de la liste. Les jalons se recalculent à la lecture du détail.
-- `law_state_kind` vaut 'none', 'in_shuttle', 'definitively_adopted' ou
-- 'became_law' ; NULL signale un dossier écrit avant ce calcul, que le
-- rafraîchissement suivant réécrit une fois, même si son sort est définitif.
ALTER TABLE legislative_dossiers ADD COLUMN law_state_kind TEXT;
ALTER TABLE legislative_dossiers ADD COLUMN law_state_date DATE;
ALTER TABLE legislative_dossiers ADD COLUMN last_decision_chamber TEXT;
ALTER TABLE legislative_dossiers ADD COLUMN last_decision_sense TEXT;
ALTER TABLE legislative_dossiers ADD COLUMN last_decision_date DATE;

CREATE INDEX idx_legislative_dossiers_law_state_kind
    ON legislative_dossiers (law_state_kind);
