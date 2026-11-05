-- Le job quotidien ne compare plus les pages de programme et d'accueil des
-- candidatures à PDF que sur leurs liens PDF, et interroge un PDF sans son ancre
-- « #page=N ». Ces observations repartent de zéro : l'ancien hash ne se compare
-- pas au nouveau et produirait une fausse alerte. Elles ne contiennent que des
-- métadonnées techniques, recalculées au passage suivant.
DELETE FROM candidate_program_source_observations
 WHERE source_url LIKE '%#%'
    OR source_url IN (SELECT official_site_url FROM presidential_candidates WHERE official_site_url IS NOT NULL)
    OR source_url IN (SELECT program_url FROM presidential_candidates WHERE program_url IS NOT NULL);
