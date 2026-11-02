# Extraits de programmes 2027 — revue avant intégration

Données : `scripts/candidates-2027-proposals.json`, lu par `scripts/seed-candidates-2027.mjs`. Pages lues, thèmes non abordés et difficultés, par candidat : `sources-lues.json` (même dossier).

**1 011 nouveaux extraits** pour 28 candidats, en plus des 84 déjà publiés. Intégrés au seed après arbitrage du 02/11 (voir en fin de note).

## Méthode

- **Sources primaires uniquement** : site du candidat ou de son parti, livret, PDF programme. Pas de presse.
- **Extrait exact** : chaque `excerpt` est copié mot pour mot. Une phrase peut être coupée à une frontière naturelle, sans reformulation.
- **Un seul thème par extrait**, dans le référentiel fermé des 16 familles, choisi sur l'objet du dispositif.
- **Au plus 5 nouveaux extraits par candidat et par thème.** On privilégie les mesures que le programme met lui-même en avant. Les cases à 6 comptent un extrait déjà publié.
- **Vérification indépendante** : 1 011 extraits sur 1 011 sont retrouvés dans leur source, re-téléchargée à part (HTML, PDF, export du Google Doc). Seuls la typographie, les espaces et les césures PDF ont été normalisés pour la comparaison.

## Couverture (publiés + nouveaux)

PA pouvoir d'achat · ECO économie/finances publiques · LOG logement · TRA travail · PSR protection sociale/retraites · SAN santé · ENV environnement/énergie · AGR agriculture · NUM numérique · JUS justice/sécurité · IMM immigration · EDU éducation/culture · SOC société/libertés · INT international/défense · INS institutions · COL collectivités/outre-mer

| Candidat | PA | ECO | LOG | TRA | PSR | SAN | ENV | AGR | NUM | JUS | IMM | EDU | SOC | INT | INS | COL | Nouveaux | Publiés |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Édouard Philippe | 3 | 5 | · | 5 | 5 | 2 | 5 | 1 | 3 | 5 | 5 | 5 | · | 5 | 3 | · | 48 | 4 |
| Nicolas Dupont-Aignan | 5 | 6 | 5 | 5 | 5 | 5 | 5 | 5 | 6 | 6 | 5 | 5 | 5 | 5 | 6 | 5 | 80 | 4 |
| Augustin Billetdoux | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | 0 | 0 |
| Delphine Batho | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | 0 | 0 |
| Nathalie Arthaud | 2 | 5 | 1 | 2 | · | · | · | · | · | · | 1 | · | · | · | 1 | · | 9 | 3 |
| Marine Tondelier | 2 | · | 2 | 3 | 2 | 5 | 6 | 2 | 3 | 4 | · | 3 | · | · | 1 | · | 31 | 2 |
| Juan Branco | 4 | 5 | 1 | 4 | 5 | 5 | 5 | 5 | 4 | 5 | 3 | 5 | 2 | 5 | 4 | 4 | 62 | 4 |
| François Ruffin | 5 | 5 | 5 | 6 | 1 | 1 | 6 | 6 | 3 | 5 | · | 5 | · | 2 | 6 | 1 | 53 | 4 |
| Bruno Retailleau | 2 | 4 | 5 | 5 | 5 | · | 6 | 5 | 5 | 1 | 4 | 5 | 1 | 2 | 6 | · | 52 | 4 |
| François Asselineau | · | 2 | · | 1 | 1 | · | · | 2 | · | · | · | · | · | 3 | 3 | 1 | 13 | 0 |
| Benoît Mathieu | · | · | · | · | · | · | · | · | · | · | · | · | · | · | 2 | · | 1 | 1 |
| David Lisnard | 1 | 4 | 2 | 6 | 5 | 5 | 5 | 5 | · | 6 | 5 | 5 | 1 | 4 | 5 | 5 | 60 | 4 |
| Clara Egger | · | · | · | · | · | · | · | · | · | · | · | · | · | · | 6 | 6 | 10 | 2 |
| Jean-Luc Mélenchon | 6 | 5 | 5 | 5 | 5 | 5 | 6 | 5 | 5 | 5 | 5 | 5 | 5 | 6 | 6 | 5 | 80 | 4 |
| Florian Philippot | 1 | 6 | · | · | 5 | 5 | 6 | 6 | 5 | 5 | 5 | 5 | 3 | 5 | 5 | 4 | 62 | 4 |
| Gabriel Attal | 6 | 5 | 2 | 6 | 5 | 1 | 5 | 5 | 4 | 5 | 3 | 6 | · | 6 | 3 | 3 | 61 | 4 |
| Jean-François Legendre | 3 | 6 | · | · | 2 | · | 5 | 3 | 1 | 5 | 5 | 4 | · | 3 | 5 | 2 | 40 | 4 |
| Anasse Kazib | 1 | 4 | 1 | 3 | 2 | · | 1 | · | · | 2 | 1 | 1 | 4 | 2 | 1 | · | 19 | 4 |
| Karim Bouamrane | · | · | 1 | · | 1 | 1 | · | · | · | 1 | 1 | 1 | · | 1 | 1 | · | 4 | 4 |
| Selma Labib | 2 | 5 | · | 3 | · | · | · | · | · | · | 1 | · | · | 1 | · | · | 9 | 3 |
| Antoine Mikolajczak | · | 6 | · | 3 | · | 5 | 5 | 5 | 3 | 5 | · | 5 | · | · | 6 | 4 | 44 | 3 |
| Marine Le Pen | 1 | · | 2 | · | 2 | · | · | · | · | 3 | 1 | · | 2 | · | 1 | · | 12 | 0 |
| Francis Lalanne | 1 | 5 | · | 1 | · | · | · | 3 | · | 3 | · | · | 2 | 6 | 4 | · | 21 | 4 |
| Raphaël Glucksmann | 5 | 6 | 5 | 6 | 6 | 5 | 6 | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 4 | 79 | 4 |
| Erwan Rohou | · | 1 | 3 | · | · | · | 6 | 4 | 2 | 3 | · | · | · | · | 5 | 2 | 23 | 3 |
| Fabien Roussel | 2 | 3 | · | 3 | 1 | · | 2 | · | · | 3 | · | 5 | 1 | 1 | 1 | · | 18 | 4 |
| Éric Zemmour | 3 | · | · | · | 1 | 1 | · | · | · | · | 2 | 1 | · | · | · | · | 8 | 0 |
| Olivier Becht | 1 | 4 | 4 | 5 | 6 | 6 | 4 | 2 | 1 | 6 | 5 | 5 | 4 | 3 | 4 | 5 | 61 | 4 |
| Mira Markovic | 5 | 5 | 1 | 1 | · | 4 | 5 | 5 | · | 5 | · | 5 | 1 | 2 | 5 | 2 | 46 | 0 |
| Natacha Polony | · | 1 | · | · | · | · | 1 | · | · | · | · | 1 | · | · | 5 | · | 5 | 3 |

Une case vide signifie qu'aucune mesure n'a été trouvée sur ce thème dans les sources lues. Elle ne vaut pas absence de position.

## À arbitrer : sources qui ne sont pas un programme présidentiel

| Candidat | Source utilisée | Extraits |
|---|---|---:|
| Marine Le Pen | `marinelepen.com` n'est qu'une page d'inscription. Extraits tirés des « mesures » à signer et de 2 pétitions de campagne sur `rassemblementnational.fr` (sept.-oct. 2026) | 12 |
| Éric Zemmour | Pas de programme 2027 (`ericzemmour.fr` = campagne 2022). Tract Reconquête « L'avenir est à nous », non daté | 8 |
| François Asselineau | Programme 2027 « en cours d'élaboration ». Pages permanentes de l'UPR. La page `upr.fr/le-frexit` date de 2017 et ses chiffres peuvent être périmés (7 lignes) | 13 |
| Mira Markovic | Seule source : programme national du Parti animaliste, version 2022 (« quinquennat 2022-2027 ») | 46 |
| Raphaël Glucksmann | `glucks2027.fr` n'a qu'une lettre. Source : « Acte I » de Place publique (PDF sur Google Drive), document de parti | 79 |
| Fabien Roussel | Pas de page programme. Plan Jeunesse, tracts et discours de la Fête de l'Humanité | 18 |
| Selma Labib | Réponse publique, discours, tract du NPA-R | 9 |
| David Lisnard | 2 extraits tirés de pages « Questions » du parti (CEE, ZFE) | 2 |
| Benoît Mathieu | L'extrait unique fusionne un intitulé et la phrase qui le suit | 1 |

**Sans extrait** : Augustin Billetdoux (vidéo seule, aucun texte) et Delphine Batho (programme « à élaborer avec vous »). Le texte européen 2024 de Génération Écologie n'a volontairement pas été utilisé.

## Défauts trouvés dans les extraits déjà publiés

- **Jean-Luc Mélenchon, 4 extraits** : les `source_url` `melenchon2027.fr/programme2025/livre/…` redirigent en 301 vers l'accueil. Le programme est désormais le PDF `wp-content/uploads/2025/02/A5-PROGRAMME-SITE.pdf`.
- **Édouard Philippe, extrait « règle d'or »** : il n'est pas exact. La source dit « … de maîtrise des déficits **via un référendum**, et ramener le déficit de plus de 5% à 2% du PIB… », et l'extrait publié omet le passage en gras.
- **Marine Tondelier, 2 extraits** : les pages source (pétition, fédération des Yvelines) n'ont pas pu être relues automatiquement. Elles sont à vérifier à la main.
- **Gabriel Attal, extrait « zéro déficit en 10 ans »** : il est classé `pouvoir-achat`, alors que le périmètre du thème indique plutôt `economie-finances-publiques`.

## Rattachements limites (choix faits, à confirmer)

- **Impôts des ménages, classés `pouvoir-achat` selon le périmètre du thème** :
  - ISF et droits de succession (Roussel, Zemmour) ;
  - imposition des dividendes (Ruffin) ;
  - abattement de 10 % des retraités (Branco).
- **Tarifs, classés `pouvoir-achat`** : gratuité des transports (Lalanne), « Ticket climat » (Glucksmann).
- **Sécurité civile, classée `justice-securite`** : bombardiers d'eau A400M (Mikolajczak), mesures pompiers (Roussel).
- **Classés `international-defense` (affaires européennes)** : dispense danoise sur l'asile et art. 55 / CEDH (Lisnard), sortie de Schengen et de l'UE (Philippot, classée `immigration`). Ces cas sont à la frontière avec `immigration` et `institutions-vie-politique`.
- **Rémunération des enseignants** : classée `travail-emploi` (fonction publique) chez Lisnard et Attal, mais `education-culture` chez Philippe. **À harmoniser.**
- **Hausse des salaires** : classée `travail-emploi` chez Labib, alors que l'extrait voisin déjà publié est en `pouvoir-achat`.

## Suivi

- **Mikolajczak** : programme détaillé annoncé le 4/11 à 10h.
- **Becht** : parties 10-11 du livre attendues les 5 et 9/11.
- **Mélenchon** : 5e édition de *L'Avenir en commun* en librairie le 6/11.
- **Retailleau** : pages Justice, Sécurité, Santé et Immigration « à venir ».
- **Tondelier** : programme complet (557 mesures) derrière le contrôle anti-bot Cloudflare de `lesecologistes.fr`. Extraits tirés des livrets PDF.

## Arbitrage du 02/11/2026

- **Toutes les sources ci-dessus sont conservées**, y compris les cas mineurs.
- **Rémunération et statut des enseignants** : classés `education-culture`. 5 extraits ont été déplacés depuis `travail-emploi` (Tondelier, Retailleau, Lisnard, Attal, Mikolajczak), AESH compris.
- **Extraits déjà publiés, corrigés dans le seed :**
  - Mélenchon : liens vers le PDF du programme, y compris `programUrl` ;
  - Philippe : texte actuel exact de la règle d'or ;
  - Attal : « zéro déficit » classé en `economie-finances-publiques`.
- **Le seed fait désormais référence** : il supprime les extraits des candidatures qu'il alimente avant de les réinsérer. Une correction ne laisse donc plus l'ancienne version en base.
