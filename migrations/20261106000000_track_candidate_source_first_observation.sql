-- Distingue une source référencée pour la première fois d'une source dont le
-- contenu a réellement changé : `last_changed_at` vaut la date de première
-- observation tant que le hash n'a pas bougé, donc un changement est
-- `last_changed_at > first_observed_at`.
ALTER TABLE candidate_program_source_observations
    ADD COLUMN first_observed_at TIMESTAMPTZ;

-- Les lignes existantes n'ont pas d'historique plus fin : elles repartent de
-- leur dernier changement connu, sans produire de fausse alerte.
UPDATE candidate_program_source_observations
   SET first_observed_at = last_changed_at;

ALTER TABLE candidate_program_source_observations
    ALTER COLUMN first_observed_at SET NOT NULL,
    ALTER COLUMN first_observed_at SET DEFAULT NOW();
