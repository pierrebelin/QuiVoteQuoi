-- Trace d'un refus de ligne « En bref » (SPEC-en-bref RM-13, RM-16) : ce que le
-- contrôle a refusé et la ligne écartée, pour comprendre un refus sans refaire
-- l'appel au modèle. Jamais affichés sur le site.
ALTER TABLE brief_lines
    ADD COLUMN rejection_detail TEXT,
    ADD COLUMN rejected_line TEXT,
    ADD CONSTRAINT brief_lines_rejection_trace CHECK (
        status = 'rejected' OR (rejection_detail IS NULL AND rejected_line IS NULL)
    );
