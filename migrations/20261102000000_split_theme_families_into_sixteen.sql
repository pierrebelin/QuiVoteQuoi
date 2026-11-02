-- Référentiel thématique porté de 13 à 16 familles (README.md §5).
--
-- Relecture des 341 textes en base, validée par le mainteneur le 2 novembre
-- 2026. Trois familles mêlaient des sujets que le lecteur cherche séparément :
--   1. « pouvoir d'achat / fiscalité » réunissait le budget de l'État et la
--      consommation des ménages : une loi de finances et un texte sur les prix
--      de l'énergie n'ont pas leur place dans le même bac ;
--   2. « santé / social » réunissait les soins et la protection sociale, alors
--      que les retraites restaient rangées sous « travail / emploi » ;
--   3. « institutions / procédure » servait de fourre-tout, collectivités et
--      volet procédural des lois de finances compris.
--
-- Le découpage porte sur l'objet des textes, jamais sur leur orientation
-- (README.md §6).
--
-- Trois codes disparaissent : pouvoir-achat-fiscalite, sante-social et
-- institutions-procedure. Aucune table qui référence `theme_families(code)` ne
-- les porte plus à la fin de cette migration ; le `DELETE` final, soumis aux
-- clés étrangères, échouerait sinon (voir `theme_assignments`,
-- `theme_proposal_families`, `candidate_program_proposals`).

-- 1. Familles conservées : périmètres et ordre d'affichage --------------------
--
-- Libellé et périmètre de logement, périmètres de travail, environnement,
-- justice et société sont ajustés à ce que les nouvelles familles leur retirent
-- (retraites, prix de l'énergie des ménages) ou leur cèdent (organisation des
-- juridictions, mémoire et reconnaissance). Agriculture, numérique, immigration, éducation et international
-- gardent leur libellé et leur périmètre : seul leur rang change.

UPDATE theme_families
   SET label = 'Logement / urbanisme / foncier',
       scope = 'Loyers, accès à la propriété, construction, locations de courte durée, urbanisme, droit de la propriété immobilière.',
       display_order = 3
 WHERE code = 'logement';

UPDATE theme_families
   SET scope = 'Droit du travail, chômage, formation professionnelle, fonction publique, dialogue social, indépendants.',
       display_order = 4
 WHERE code = 'travail-emploi';

UPDATE theme_families
   SET scope = 'Transition et production d''énergie, transports, eau, biodiversité, déchets.',
       display_order = 7
 WHERE code = 'environnement-energie';

UPDATE theme_families SET display_order = 8 WHERE code = 'agriculture-alimentation';
UPDATE theme_families SET display_order = 9 WHERE code = 'numerique';

UPDATE theme_families
   SET scope = 'Droit pénal, police, gendarmerie, prisons, terrorisme, sécurité civile, organisation des juridictions ; procédure judiciaire lorsqu''elle est l''objet du texte.',
       display_order = 10
 WHERE code = 'justice-securite';

UPDATE theme_families SET display_order = 11 WHERE code = 'immigration';
UPDATE theme_families SET display_order = 12 WHERE code = 'education-culture';

UPDATE theme_families
   SET scope = 'Égalité, droits des personnes, fin de vie, bioéthique, laïcité, libertés publiques, mémoire et reconnaissance. Rattachement sur l''objet du texte, jamais sur son orientation.',
       display_order = 13
 WHERE code = 'societe-libertes';

UPDATE theme_families SET display_order = 14 WHERE code = 'international-defense';

-- 2. Les six nouvelles familles ----------------------------------------------

INSERT INTO theme_families (code, label, scope, display_order) VALUES
    ('pouvoir-achat', 'Pouvoir d''achat',
     'Prix et tarifs, dont ceux de l''énergie, protection du consommateur, frais et services bancaires aux particuliers, impôts et taxes des ménages. Rattachement sur ce que fixe le dispositif, jamais sur l''effet annoncé.', 1),
    ('economie-finances-publiques', 'Économie / finances publiques',
     'Budget et comptes de l''État, lois de finances, fiscalité des entreprises, droit des sociétés, commerce, concurrence, banque et marchés financiers, commande publique, industrie.', 2),
    ('protection-sociale-retraites', 'Protection sociale / retraites',
     'Retraites, prestations et minima sociaux, politique familiale, protection de l''enfance, handicap, dépendance, financement de la sécurité sociale.', 5),
    ('sante', 'Santé',
     'Accès aux soins, hôpital, médicament, professions de santé, prévention et santé publique.', 6),
    ('institutions-vie-politique', 'Institutions / vie politique',
     'Constitution, Parlement, motions de censure, élections, statut des élus, organisation de l''État.', 15),
    ('collectivites-outre-mer', 'Collectivités / outre-mer',
     'Décentralisation, organisation et finances des collectivités, statuts de l''outre-mer et de la Corse.', 16);

-- 3. Les trois codes retirés sont renommés sur l'historique ---------------------
--
-- Chaque ancien code a un successeur unique, et l'ancienne ligne devient une
-- ligne du successeur : dates d'ouverture et de clôture, auteurs et motifs sont
-- conservés tels quels, lignes ouvertes comme lignes closes (RM-07). Rien n'est
-- supprimé ni réécrit en dehors du code de la famille.
--
--   pouvoir-achat-fiscalite -> pouvoir-achat
--   sante-social            -> sante
--   institutions-procedure  -> institutions-vie-politique
--
-- C'est un renommage provisoire, pas un reclassement. « Fiscalité » et
-- « budget » relèvent désormais d'une autre famille que « pouvoir d'achat », et
-- « social » d'une autre que « santé » : un rattachement renommé peut donc
-- n'être plus le bon. Le reclassement texte par texte suivra, par arbitrage
-- humain via l'API, sur la fiche de chaque texte (RM-07). Cette migration ne
-- nomme volontairement aucun texte.
--
-- Le renommage est une bijection vers des codes qui n'existaient pas : il ne
-- peut créer ni doublon ouvert dans `theme_assignments` (index unique sur sujet
-- et famille), ni doublon de clé dans `theme_proposal_families` ou
-- `candidate_program_proposals`, et il ne change pas le nombre de familles d'un
-- objet (RM-03).

CREATE TEMPORARY TABLE retired_families (
    old_code TEXT PRIMARY KEY,
    new_code TEXT NOT NULL
) ON COMMIT DROP;

INSERT INTO retired_families (old_code, new_code) VALUES
    ('pouvoir-achat-fiscalite', 'pouvoir-achat'),
    ('sante-social', 'sante'),
    ('institutions-procedure', 'institutions-vie-politique');

UPDATE theme_assignments a
   SET family_code = m.new_code
  FROM retired_families m
 WHERE a.family_code = m.old_code;

UPDATE theme_proposal_families p
   SET family_code = m.new_code
  FROM retired_families m
 WHERE p.family_code = m.old_code;

UPDATE candidate_program_proposals c
   SET family_code = m.new_code
  FROM retired_families m
 WHERE c.family_code = m.old_code;

-- 4. Rattachements par règle publiée : refaits sur la table de règles neuve ------
--
-- Le renommage ci-dessus ne convient pas aux textes réglés. Une règle tranche
-- sur la nature juridique du texte, pas sur un jugement : ses familles se
-- déduisent de la table publiée (src/domain/theme_rules.rs), qui change pour
-- huit règles :
--
--   financement de la sécurité sociale    -> protection sociale + santé
--   comptes de la sécurité sociale        -> protection sociale + santé
--   loi de finances                       -> économie / finances publiques
--   règlement du budget                   -> économie / finances publiques
--   programmation des finances publiques  -> économie / finances publiques
--   règlement de l'Assemblée              -> institutions / vie politique
--   motion de censure                     -> institutions / vie politique
--   fraudes sociales et fiscales          -> protection sociale + économie
--
-- Une loi de finances n'est plus rattachée d'office aux institutions : la
-- procédure budgétaire n'est pas l'objet du texte.
--
-- Pour les textes dont un rattachement ouvert a pour auteur une de ces règles,
-- on fait comme `replace_assignments` quand une règle s'applique : les lignes
-- ouvertes sont closes à la date de la migration, puis les familles de la
-- nouvelle table sont ouvertes à la même date, avec le même auteur
-- (`règle « <id> »`) et la même justification, reprise de la ligne close —
-- l'énoncé des règles ne change pas. L'historique garde l'ancien état.
--
-- Les autres règles gardent leurs familles : leurs lignes restent ouvertes.
-- Un rattachement arbitré par un humain ou proposé par le modèle n'a pas pour
-- auteur une règle : il n'est pas touché, et sera repris texte par texte.

CREATE TEMPORARY TABLE rule_families (
    rule_id TEXT NOT NULL,
    ordinal SMALLINT NOT NULL,
    family_code TEXT NOT NULL,
    PRIMARY KEY (rule_id, ordinal)
) ON COMMIT DROP;

INSERT INTO rule_families (rule_id, ordinal, family_code) VALUES
    ('financement de la sécurité sociale', 1, 'protection-sociale-retraites'),
    ('financement de la sécurité sociale', 2, 'sante'),
    ('comptes de la sécurité sociale', 1, 'protection-sociale-retraites'),
    ('comptes de la sécurité sociale', 2, 'sante'),
    ('loi de finances', 1, 'economie-finances-publiques'),
    ('règlement du budget', 1, 'economie-finances-publiques'),
    ('programmation des finances publiques', 1, 'economie-finances-publiques'),
    ('règlement de l''Assemblée', 1, 'institutions-vie-politique'),
    ('motion de censure', 1, 'institutions-vie-politique'),
    ('fraudes sociales et fiscales', 1, 'protection-sociale-retraites'),
    ('fraudes sociales et fiscales', 2, 'economie-finances-publiques');

CREATE TEMPORARY TABLE ruled_subjects ON COMMIT DROP AS
SELECT a.subject_kind, a.subject_id, a.author, MAX(a.motive) AS motive
  FROM theme_assignments a
 WHERE a.closed_on IS NULL
   AND a.author IN (SELECT 'règle « ' || rule_id || ' »' FROM rule_families)
 GROUP BY a.subject_kind, a.subject_id, a.author;

-- `GREATEST` protège la contrainte de dates quand un rattachement a été ouvert
-- le jour même : on ne clôt jamais avant l'ouverture (RM-07).
UPDATE theme_assignments a
   SET closed_on = GREATEST(CURRENT_DATE, a.opened_on)
  FROM ruled_subjects r
 WHERE a.closed_on IS NULL
   AND a.subject_kind = r.subject_kind
   AND a.subject_id = r.subject_id
   AND a.author = r.author;

INSERT INTO theme_assignments (subject_kind, subject_id, family_code, opened_on, author, motive)
SELECT r.subject_kind, r.subject_id, f.family_code, CURRENT_DATE, r.author, r.motive
  FROM ruled_subjects r
  JOIN rule_families f ON r.author = 'règle « ' || f.rule_id || ' »'
 ORDER BY r.subject_kind, r.subject_id, f.ordinal;

-- 5. Retrait des trois anciennes familles ------------------------------------
--
-- Plus aucune ligne ne les référence : les clés étrangères le vérifient. Si une
-- table non prévue ici les portait encore, ce `DELETE` fait échouer la
-- migration, plutôt que de laisser un rattachement orphelin.

DELETE FROM theme_families
 WHERE code IN ('pouvoir-achat-fiscalite', 'sante-social', 'institutions-procedure');
