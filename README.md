# QuiVoteQuoi

**Consulter ce que les groupes parlementaires ont réellement voté à l'Assemblée nationale — par thème, sur pièces, sans interprétation.**

Les votes de l'Assemblée sont publics, horodatés et opposables. Ils sont aussi dispersés dans des jeux de données que presque personne n'ouvre. Cet outil rapproche les deux bouts : *voici les textes sur le logement, voici comment chaque groupe a voté, voici la source officielle*.

Backend Rust (API REST), frontend SPA React/TypeScript, données issues de l'open data de l'Assemblée nationale.

---

## Dépôt public

Le code source, les migrations et l'historique des évolutions sont consultables
sur [GitHub](https://github.com/pierrebelin/QuiVoteQuoi), sous
[PolyForm Noncommercial 1.0.0](LICENSE). Il est donc public et modifiable pour
des usages non commerciaux, mais ce n'est pas une licence open source au sens
strict. Cette transparence permet de vérifier les règles de traitement ; pour
les votes et leurs chiffres, la source officielle de l'Assemblée nationale
reste la référence.

Le dépôt ne contient ni secrets, ni accès à l'infrastructure, ni export de base
de données, ni données personnelles. Les configurations locales partent de
`.env.example` ; le fichier `.env` reste ignoré par Git.

Ce document est aussi la **charte du produit**. Ses sections sont numérotées et le code y renvoie directement (`README.md §6`) : une règle citée ici est une contrainte d'implémentation, pas une intention.

## 1. Intention

Le débat électoral se joue sur des déclarations. Les actes de vote, eux, sont vérifiables. L'outil sert à confronter les uns aux autres.

- **Public visé** : électeurs qui veulent vérifier un discours contre un acte. Curieux, peu experts.
- **Promesse** : « sur ce sujet, voici les votes — chiffres bruts, source cliquable ».
- **Anti-promesse** : aucune note, aucun classement, aucun jugement sur un groupe.
- **Posture** : instrument de vérification, pas média d'opinion.

Le test qualité est constant : *une personne de droite et une personne de gauche trouveraient-elles la page juste ?*

## 2. Règle structurante — exhaustivité

C'est la contrainte la plus lourde du produit, et elle n'est pas négociable.

Sur un site de transparence en période électorale, **le choix de ce qu'on montre est déjà un acte éditorial**. Afficher 30 votes sur le logement et en taire 12 est indéfendable, quelle que soit la raison technique.

Conséquences directes :

- On ingère et on expose **tous** les scrutins de la législature, sans sélection.
- Le scoring **ordonne** l'affichage (pertinence, ampleur), il ne **filtre** jamais.
- Tout critère de classement est **public et consultable** sur le site.
- Toute lacune connue (source indisponible, vote à main levée) est **affichée comme telle**, jamais silencieuse.

## 3. Modèle politique — les trois pièges

### 3.1 Groupe parlementaire ≠ parti

L'Assemblée publie des **groupes**. Le public raisonne en **partis**. Ce ne sont pas les mêmes objets : certains groupes rassemblent plusieurs partis, des députés y sont rattachés sans en être membres, et certains partis n'ont aucun groupe.

**Règle** : on affiche le groupe, nommé comme tel, avec une page de composition. Aucune traduction silencieuse groupe → parti. Une équivalence approximative présentée comme un fait est une fausse information.

### 3.2 Appartenance datée

Un député change de groupe en cours de législature. Un vote de 2024 doit être compté avec le groupe du député **à la date du vote**, jamais avec son groupe actuel.

**Règle** : l'appartenance est stockée avec ses dates de validité ; toute agrégation de votes joint sur l'appartenance à la date du scrutin. Une jointure sur l'appartenance courante réécrit l'histoire des votes.

### 3.3 Candidat ≠ député

Beaucoup de personnalités en lice n'ont jamais siégé à l'Assemblée, ou plus depuis plusieurs législatures. Leur absence de votes n'est pas une information sur elles.

**Règle** : le site ne présente jamais « la position de X » à partir de votes parlementaires. Il présente « les votes du groupe Y ». Une page de programme peut montrer, séparément, des extraits attribués à une candidature déclarée et les votes d'un groupe que des sources relient explicitement à cette candidature ; elle ne conclut jamais à un alignement. La limite est affichée sur les pages concernées, pas enfouie dans une FAQ.

## 4. Architecture fonctionnelle

```
Ingestion continue AN (dossiers + scrutins + députés + appartenances)
  → Base de faits horodatés
  → Thématisation (rattachement texte débattu → familles, hérité par scrutins et dossiers)
  → Pages structurées : thème × groupe × période
  → Chat (couche de routage, sans production de chiffres)
```

- L'ingestion tourne **au fil de l'eau**, sans validation humaine préalable : la donnée est brute et sourcée, il n'y a rien à valider éditorialement.
- Le point de contrôle humain porte sur **la thématisation** (§5) et sur la **qualité de l'ingestion**, pas sur le contenu affiché.

## 5. Thématisation

Rattacher un texte à un ou plusieurs thèmes est **le seul endroit où un jugement entre dans le produit**. Il est donc traité comme tel : critères explicites, résultat inspectable, correction possible.

**Le porteur du thème est le texte débattu, pas le dossier.** La source publie le lien vers le dossier de façon irrégulière *à l'intérieur d'un même texte* : la proposition de loi relative au droit à l'aide à mourir porte 297 scrutins avec dossier et 541 sans. Thématiser le dossier laisserait 541 votes du même texte hors thème — exactement la sélection éditoriale que §2 interdit.

Le texte est extrait de l'objet du scrutin par une règle déterministe : 8 428 scrutins sur 8 434 se regroupent en 322 textes. Un scrutin hérite des familles de son texte ; un dossier hérite du texte que ses propres scrutins nomment, et n'est classé directement que s'il n'a aucun scrutin.

Familles cibles — **seize**, référentiel fermé :

- Pouvoir d'achat (prix et tarifs, dont ceux de l'énergie, protection du consommateur, frais bancaires des particuliers, impôts et taxes des ménages)
- Économie / finances publiques (budget et comptes de l'État, lois de finances, fiscalité des entreprises, droit des sociétés, commerce, concurrence, banque et marchés financiers, commande publique, industrie)
- Logement / urbanisme / foncier (loyers, accès à la propriété, construction, locations de courte durée, urbanisme, droit immobilier)
- Travail / emploi (droit du travail, chômage, formation professionnelle, fonction publique, dialogue social, indépendants)
- Protection sociale / retraites (retraites, prestations et minima sociaux, politique familiale, protection de l'enfance, handicap, dépendance, financement de la sécurité sociale)
- Santé (accès aux soins, hôpital, médicament, professions de santé, prévention et santé publique)
- Environnement / énergie (transition et production d'énergie, transports, eau, biodiversité, déchets)
- Agriculture / alimentation (revenu agricole, pêche, phytosanitaires, foncier agricole)
- Numérique (données, IA, réseaux sociaux, fraude en ligne)
- Justice / sécurité (pénal, police, gendarmerie, prisons, terrorisme, sécurité civile, organisation des juridictions ; procédure judiciaire lorsqu'elle est l'objet du texte)
- Immigration (séjour, asile, éloignement, nationalité — **terrain sensible : factuel uniquement**)
- Éducation / culture (école, université, recherche, sport, culture, médias)
- Société / libertés (égalité, droits des personnes, fin de vie, bioéthique, laïcité, libertés publiques, mémoire et reconnaissance — **terrain sensible : factuel uniquement**)
- International / défense (ratification de traités, armées, aide au développement, Europe)
- Institutions / vie politique (Constitution, Parlement, motions de censure, élections, statut des élus, organisation de l'État)
- Collectivités / outre-mer (décentralisation, organisation et finances des collectivités, statuts de l'outre-mer et de la Corse)

Le référentiel est passé de huit à treize familles le 9 août 2026. Les huit précédentes concentraient justice, sécurité, immigration, éducation et culture dans « société / libertés » — les sujets les plus disputés de la législature dans le bac qu'on avait soi-même marqué sensible — et n'accueillaient ni l'international ni la défense.

Il est passé de treize à seize le 2 novembre 2026, après relecture des 341 textes en base. Trois familles mêlaient des sujets qu'un lecteur cherche séparément : « pouvoir d'achat / fiscalité » réunissait le budget de l'État et la consommation des ménages ; « santé / social » réunissait les soins et la protection sociale, retraites comprises ; « institutions / procédure » servait de fourre-tout, collectivités et volet procédural des lois de finances compris. Elles deviennent « pouvoir d'achat » et « économie / finances publiques », « santé » et « protection sociale / retraites », « institutions / vie politique » et « collectivités / outre-mer ». Les rattachements existants ont d'abord été renommés sur leur historique (dates, auteurs et motifs conservés) ; le reclassement texte par texte se fait ensuite par arbitrage, famille par famille.

Dans les deux cas, le découpage porte sur l'**objet** des textes ; il ne dit rien de leur orientation (§6).

Règles :

- Un texte peut appartenir à **plusieurs** familles, trois au plus ; on ne force pas l'unicité.
- On rattache sur le **sujet du texte**, jamais sur le code qu'il modifie ni sur le public qu'il vise : un texte qui touche au code de la sécurité sociale ou qui s'adresse aux étudiants relève de ce qu'il fixe, pas de son véhicule ni de ses destinataires.
- Une **résolution européenne** est rattachée sur son dispositif — ce qu'elle demande ou propose —, pas sur ses considérants.
- Le rattachement est **révisable** et son historique conservé.
- Un texte non rattaché reste **consultable** (liste dédiée), il n'est pas perdu.
- La méthode de rattachement est publiée sur le site, table de règles comprise.
- Trois chemins mènent à un rattachement : une **règle publiée** tranche quand la nature juridique du texte suffit (un projet de loi de finances est un texte budgétaire) ; sinon le modèle **propose** ; un humain peut **arbitrer** après coup, et son arbitrage prime. Le modèle ne voit que le libellé du texte.
- **Le rattachement ne s'affiche pas avec une étiquette d'origine.** Le site montre la famille, pas qui l'a décidée : lire « logement — auto » plutôt que « logement » déplace l'attention sans rien apprendre sur le vote. La traçabilité reste entière — l'auteur de chaque rattachement (règle, modèle ou mainteneur), sa justification et l'historique des révisions sont consultables sur la fiche du texte (§9).

**Économie d'appels.** Le rattachement est le seul poste du produit qui appelle un modèle, donc le seul qui coûte à l'usage. Trois leviers, dans cet ordre : le porteur est le texte et non le scrutin (8 434 scrutins tiennent en 322 textes, et scrutins comme dossiers en héritent) ; les règles publiées prennent ce qu'elles savent prendre sans un jeton ; le reste part au modèle **par lot**, le cadrage n'étant payé qu'une fois par lot. Un objet déjà rattaché n'est jamais resoumis. Un rafraîchissement de routine ne coûte donc que ce qui est nouveau, pas la taille de la base.

## 6. Neutralité — règles dures

- **Faits uniquement** : quel texte, ce qu'il prévoit, qui a voté quoi, résultat, étape.
- **Aucune évaluation du texte** (bon/mauvais, juste/injuste, efficace/non).
- **Aucune évaluation d'un groupe** : pas de score de cohérence, pas de taux de présence présenté comme un mérite, pas d'agrégat qui se lit comme un classement.
- **Des nombres, pas d'adverbes** : « rejeté par 280 voix contre 250 » ✅ ; « massivement rejeté » ❌.
- **Positions attribuées et on-record uniquement** : exposé des motifs, interventions en séance, position de vote ; pour une candidature déclarée, extrait exact de son programme primaire. Jamais de commentaire média ni de synthèse libre.
- **Lien vers la source officielle sur chaque chiffre affiché.**
- **Le LLM ne produit jamais un chiffre** (§8).

## 7. Traitement des votes

- **Répartition par groupe** (pour / contre / abstention / non-votants), chiffres bruts.
- **Vote sur l’ensemble** : un « vote final » désigne ici un scrutin dont l’objet officiel porte sur l’ensemble d’un texte. Il tranche la version soumise à cette lecture, pas nécessairement le devenir final de la loi. Un même texte peut donc avoir plusieurs votes sur l’ensemble.
- **Vote le plus récent d’un dossier** : lorsqu’un aperçu doit n’en retenir qu’un, il prend le vote sur l’ensemble le plus récent parmi ceux que la source rattache explicitement à ce dossier (date décroissante, puis numéro de scrutin publié). Les autres lectures restent accessibles. Aucun lien dossier ↔ scrutin n’est déduit d’une ressemblance de titres ; un scrutin sans dossier reste affiché comme tel.
- **Position nominale par député** disponible au détail du scrutin — c'est le niveau de preuve qui rend le site vérifiable.
- **Trou de couverture, pas second régime** : tous les scrutins publiés par l'Assemblée portent le décompte nominatif. Les votes à main levée ne sont pas publiés *sans* répartition — ils sont **absents du jeu de données**. Le site ne peut donc rien en dire. Cette lacune est **affichée comme telle** sur les pages concernées ; la taire laisserait croire à une exhaustivité que la source ne permet pas.
- **Scrutins sans dossier** : conservés et exposés. **69 % des scrutins de la législature en cours ne portent aucun rattachement à un dossier** — les écarter viderait le site des deux tiers des votes.
- **Ne jamais inventer de chiffres.**

## 8. Interfaces

### 8.1 Pages structurées — le socle

URL stables et partageables : thème, groupe, dossier, scrutin, député. Servies directement depuis la base : aucun risque d'invention, indexables, citables.

La fiche dossier comporte aussi une vue « Actes publiés par groupe ». Elle
réunit, dans un ordre stable et sans classement, les groupes dont la période
d'existence recouvre celle du dossier, les seuls votes finaux séparés par
lecture, et les amendements associés à la date du dépôt. Un groupe sans acte
reste affiché avec cet état explicite : son absence de ligne ne devient jamais
une abstention.

Un court paragraphe peut être publié sous le label **« Synthèse automatique »**.
Il est généré après ingestion à partir d'un paquet fermé de faits officiels et
d'identifiants de sources. Il est descriptif uniquement : aucun chiffre,
classement, comparaison, évaluation, intention, causalité ou position globale
de groupe ne peut y figurer. Les chiffres sont calculés et rendus par le code,
et les sources originales restent accessibles dans les listes exhaustives
« Scrutins » et « Amendements ».

Cette synthèse ne remplace donc jamais le verbatim des exposés sommaires. Les
déclarations externes de partis ou de groupes restent hors périmètre, sauf les
sources primaires explicitement publiées dans la page de programme décrite ci-dessous.

### 8.2 Programmes déclarés 2027

La page « Programmes et votes associés — 2027 » recense les candidatures dont une déclaration publique primaire est disponible. Pour chaque candidat, elle publie la déclaration, les organisations ou soutiens explicitement sourcés, le lien vers le programme et des extraits attribués à celui-ci, rattachés au référentiel fermé des thèmes.

- Un extrait porte toujours son lien source et sa date lorsqu'elle est publiée.
- Parti, soutien de campagne et groupe parlementaire sont des objets distincts. Un groupe n'est associé à une candidature que lorsqu'une source le dit ; son vote reste celui du groupe, à la date du scrutin.
- La comparaison est côte à côte : aucun score, étiquette « aligné / non aligné », résumé libre ni position personnelle déduite n'est produit.
- Tant qu'aucune déclaration vérifiable n'est enregistrée, la page affiche explicitement cet état au lieu de données de démonstration.

### 8.3 Chat — surcouche

Le chat **route et cite**, il ne rédige pas les faits.

- Le LLM interprète la demande (thème + groupe + période) et sélectionne les données.
- Les chiffres affichés proviennent de la base, jamais du texte généré.
- Chaque réponse renvoie vers les pages et les sources officielles.

Motif : une seule hallucination sur un chiffre de vote détruit la crédibilité de l'ensemble, et c'est irrattrapable en période électorale.

## 9. Défense méthodologique

Un site qui rapproche votes et forces politiques avant une présidentielle sera contesté sur sa méthode. La défense se construit en amont, pas après :

- Méthode de thématisation et de tri **publiée**.
- Chaque chiffre **traçable** jusqu'à la source officielle.
- Données sous **Licence Ouverte** (open data AN), réutilisation licite.
- Corrections **journalisées** publiquement.
- Aucun agrégat comparatif entre groupes qui ressemble à une note.

## 10. Sources de données

### Socle Assemblée nationale — `data.assemblee-nationale.fr`

Colonne vertébrale. XML / JSON. Jeux clés :

- **Scrutins** : position de vote de chaque député (→ §7). Priorité absolue.
- **Acteurs et organes** : députés, groupes, **mandats et appartenances datées** (→ §3.2).
- Dossiers législatifs (titres, dépôts, commissions, rapporteurs, dates).
- **Amendements** : auteur, cosignataires, **exposé sommaire**, sort. Ingérés depuis l'archive de la législature. Ce sont les seules raisons de vote que la source publie sous forme structurée — il n'existe aucun champ « raison du vote » dans le jeu des scrutins.
- Comptes rendus de séance (positions attribuées). Non ingérés à ce jour.

### Vie publique — `vie-publique.fr`

- « Panorama des lois » : résumés en langage clair et neutre, aide à la thématisation.
- Depuis avril 2026, l'accès aux dossiers législatifs passe par Vie publique (redirections depuis Légifrance).

### Légifrance via PISTE

API officielle : textes promulgués, droit en vigueur, Journal officiel. Auth OAuth, sandbox + production.

### Couches tierces (vérifier la pérennité avant d'en dépendre)

- **CIVIX** : agrège scrutins, votes individuels, groupes, dossiers, sans interprétation.
- **Tricoteuses** (`@tricoteuses/assemblee`) : données AN nettoyées.
- **NosDéputés.fr / ParlAPI.fr** (Regards Citoyens) : recherche plein texte.

### Sénat — `data.senat.fr`

**Hors périmètre initial**, choix assumé : l'Assemblée est élue au suffrage universel direct, ses votes sont les plus lisibles pour l'usage visé. À rouvrir si besoin.

## 11. Stack

- **Backend Rust** — Axum, Tokio, sqlx, serde.
- **Frontend React/TypeScript** — Vite, SPA, TanStack Query, Tailwind.
- **Base** : PostgreSQL local sur le VPS.
- Clean Architecture + DDD, couche application découpée par use case. Les consignes détaillées vivent dans les fichiers locaux `.agents/`.
- LLM en BYOK, cantonné à la thématisation et aux synthèses descriptives
  strictement bornées — **jamais à la production de chiffres**.

## 12. Feuille de route

- [x] **Phase 0** — Repo, ingestion des dossiers législatifs AN.
- [x] **Phase 1** — Enrichissement du domaine dossier (étape, initiateurs, commission, documents, résumé).
- [x] **Phase 2** — **Acteurs** : référentiel députés et groupes, appartenances datées (§3.2).
- [x] **Phase 3** — **Scrutins** : ingestion, positions nominales, répartition par groupe.
- [x] **Phase 4** — **Thématisation** : textes débattus, familles, méthode publiée (§5).
- [x] **Phase 4bis** — **Amendements** : ingestion, rattachement au dossier, exposé sommaire verbatim sur la page dossier.
- [x] **Phase 4ter** — **Actes par groupe** : votes finaux par lecture, amendements associés et synthèse automatique descriptive pré-calculée.
- [ ] **Phase 5** — Pages publiques thème × groupe × période.
- [ ] **Phase 5bis** — Programmes déclarés 2027 : candidatures, extraits sourcés et accès séparé aux votes des groupes associés.
- [ ] **Phase 6** — Chat de routage (§8.2).
- [ ] **Phase 7** — Page méthodologie, journal des corrections (§9).

## Non-objectifs

- Pas de note, de classement ni de score attribué à un groupe ou à une personne.
- Pas d'opinion, pas de conclusion éditoriale, pas de recherche de viralité.
- Pas de traduction groupe → parti sans base factuelle.
- Pas de prédiction électorale, pas de comparaison de programmes.
- Pas de chiffre produit par un LLM.

---

## Démarrage local

Prérequis : Rust stable, Node 22+, une base Postgres accessible.

```bash
cp .env.example .env
```

Renseigner `DATABASE_URL` dans `.env`. Les migrations sont jouées automatiquement au démarrage du binaire.

Variables reconnues :

| Variable | Rôle |
|---|---|
| `DATABASE_URL` | connexion Postgres (obligatoire) |
| `PORT` | port d'écoute de l'API (défaut `3000`) |
| `BIND_ADDR` | adresse d'écoute (défaut `127.0.0.1`) |
| `ADMIN_TOKEN_SECRET` | secret maître des routes d'écriture ; absent, toute écriture est fermée |
| `ALLOWED_ORIGINS` | origines tierces autorisées par CORS, séparées par des virgules ; vide par défaut |
| `LLM_PROVIDER` | fournisseur des traitements LLM : `anthropic`/`claude`, `openai`/`chatgpt` ou `mistral` ; défaut `anthropic` |
| `ANTHROPIC_API_KEY` | clé Claude, utilisée seulement si `LLM_PROVIDER=anthropic` |
| `OPENAI_API_KEY` | clé OpenAI/ChatGPT, utilisée seulement si `LLM_PROVIDER=openai` |
| `MISTRAL_API_KEY` | clé Mistral, utilisée seulement si `LLM_PROVIDER=mistral` |
| `ANTHROPIC_MODEL` / `OPENAI_MODEL` / `MISTRAL_MODEL` | modèle par défaut du fournisseur correspondant (Mistral : `mistral-small-latest`, un identifiant daté est préférable) |
| `THEME_MODEL` | surcharge facultative du modèle de thématisation |
| `AMENDMENT_BATCH_PER_REFRESH` | amendements écrits par passe d'ingestion (défaut `40000`) ; `0` lève la borne |
| `DOSSIER_SUMMARY_BATCH_PER_REFRESH` | dossiers examinés pour les synthèses après ingestion (défaut `10`) |
| `DOSSIER_SUMMARY_MODEL` | surcharge facultative du modèle de synthèse ; à défaut, modèle du fournisseur sélectionné |

Le fournisseur se change dans `.env`, puis le service doit être redémarré. Les
adaptateurs envoient le même paquet de faits et appliquent les mêmes
validations ; aucune clé n'est utilisée en repli automatique. Une clé absente
ou une erreur d'appel laisse les données brutes accessibles et les synthèses
indisponibles, sans faire échouer l'ingestion.

### Routes d'écriture

Les routes de consultation sont ouvertes : le site publie de la donnée
publique, et un jeton embarqué dans un bundle JavaScript public ne protégerait
rien — pas plus que `Origin` ou `Referer`, qu'un `curl` falsifie.

Les routes d'écriture (ingestion, rattachement thématique, synchronisation)
exigent en revanche un jeton, présenté en `x-admin-token` ou en
`Authorization: Bearer`.
Ce jeton **change chaque jour** : il est dérivé de `ADMIN_TOKEN_SECRET` et de la
date UTC, jamais stocké. Le serveur accepte celui du jour et celui de la veille,
pour qu'une tâche lancée avant minuit ne tombe pas en 401 après.

```bash
# jeton du jour, à coller dans l'écran d'administration
ADMIN_TOKEN_SECRET=... cargo run --bin admin-token
```

Un jeton qui fuite meurt en 48 h au plus. Pour révoquer immédiatement : changer
`ADMIN_TOKEN_SECRET` et redémarrer.

Backend :

```bash
cargo run
```

Frontend, dans un second terminal :

```bash
cd frontend && npm ci && npm run dev
```

Le serveur de développement Vite relaie `/api` vers `http://localhost:3000` (surchargeable via `VITE_API_TARGET`).

Tests :

```bash
cargo test
cd frontend && npm run lint && npm run build
```

Les tests de use case reposent sur des fakes in-memory : aucune base n'est nécessaire.

## Contribution

Les règles des sections §1 à §12 encadrent toute évolution. Une contribution qui ajoute un jugement, un classement ou un filtrage de votes sera refusée sur ce motif, même bien intentionnée. Toute contribution est proposée sous [PolyForm Noncommercial 1.0.0](LICENSE) ; son auteur confirme avoir le droit de la soumettre. Avant toute proposition, vérifier les tests Rust et le lint puis le build du frontend.

Ne jamais versionner de secret, d'adresse d'hôte ni de donnée personnelle — voir « Dépôt public » ci-dessus.

## Licence et données

Le **code du projet** est distribué sous [PolyForm Noncommercial 1.0.0](LICENSE) : il peut être consulté, modifié et partagé pour des usages non commerciaux. Tout usage commercial requiert une autorisation distincte de Pierre Belin.

Les **données de l'Assemblée nationale** restent sous **Licence Ouverte / Open Licence (Etalab)**, qui autorise leur réutilisation, y compris commerciale, sous ses propres conditions. Ce dépôt les réutilise sans les modifier ; leur source officielle est citée sur chaque page. La licence du code ne modifie pas le statut de ces données.
