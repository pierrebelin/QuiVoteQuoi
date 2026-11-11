-- Curation éditoriale abandonnée avec le pivot d'août 2026 (AGENTS.md) :
-- plus aucune route ne lit ni n'écrit cet état.
DROP INDEX IF EXISTS idx_legislative_dossiers_curation;

ALTER TABLE legislative_dossiers
    DROP COLUMN IF EXISTS curation_status;
