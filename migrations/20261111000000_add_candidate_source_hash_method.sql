-- Le job quotidien ne hache plus le HTML brut d'une page, qui change à chaque
-- requête (nonces, jetons Cloudflare, chronométrages, noms de fichiers de
-- build), mais son texte visible normalisé. Chaque empreinte porte désormais sa
-- méthode : quand elle change, le job remplace l'empreinte sans toucher
-- `last_changed_at`, au lieu de signaler toutes les sources d'un coup.
--
-- Les lignes existantes restent sans méthode (NULL) : leur prochaine
-- observation les rebase une fois, sans alerte. L'historique (`first_observed_at`,
-- `last_changed_at`) est conservé tel quel.
ALTER TABLE candidate_program_source_observations
    ADD COLUMN content_hash_method TEXT
        CHECK (content_hash_method IS NULL
               OR content_hash_method IN ('raw', 'pdf-links', 'visible-text-v1'));

-- Un PDF garde son empreinte octet pour octet : ses lignes reçoivent tout de
-- suite leur méthode et restent comparées dès le prochain passage.
UPDATE candidate_program_source_observations
   SET content_hash_method = 'raw'
 WHERE content_sha256 IS NOT NULL
   AND split_part(source_url, '#', 1) ILIKE '%.pdf';
