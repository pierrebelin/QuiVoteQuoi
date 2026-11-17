# Budget de l'État voté (LFI)

Un exercice = trois fichiers, versionnés tels quels :

- `lfi-<année>-credits-ae-cp.xls` : « Crédits AE et CP votés », budget.gouv.fr
  (Documentation › Exercice <année> › Loi de finances initiale). Crédits de
  paiement par mission, programme, action et catégorie. L'import garde les
  actions et leur nature de dépense (catégorie, dont le premier chiffre est le
  titre de la LOLF, art. 5) ; il refuse un titre contraire à la catégorie et des
  actions dont la somme diffère des CP de leur programme.
- `lfi-<année>-nomenclature-destination.xls` : « Nomenclature par
  destination », même page. Seule source des programmes dotés de 0 €, que le
  fichier des crédits omet (README §2 : tous les programmes sont montrés).
- `lfi-<année>.json` : le manifeste. Loi (numéro, date, article des crédits,
  procédure d'adoption, lien Légifrance), URL et dates des deux fichiers, leurs
  empreintes SHA-256, et les attendus tirés de la loi : total des crédits de
  paiement du budget général (article de crédits, état B), nombre de missions
  et de programmes (état B), comptes hors budget retenus et leur total (état D).

Les attendus se recopient depuis le texte de la loi, jamais depuis les fichiers
eux-mêmes, ni d'une maquette, ni d'un LLM (README §8) : un fichier faux de la
même façon que ses propres totaux passerait sinon tous les contrôles.

budget.gouv.fr refuse les téléchargements automatisés : les fichiers se
récupèrent depuis un navigateur. `shasum -a 256 <fichier>` donne l'empreinte.

## Ajouter un exercice

1. Télécharger les deux fichiers de la LFI, les nommer comme ci-dessus.
2. Écrire `lfi-<année>.json` sur le modèle de `lfi-2026.json`.
3. `cargo run --bin import-budget -- data/budget/lfi-<année>.json --dry-run`.
   Une mission nouvelle, sans sujet, est refusée : la rattacher dans
   `MISSION_FAMILIES` (`src/domain/budget.rs`) après décision.
   Un nombre de programmes supérieur à l'état B vient en général d'un programme
   clos que la nomenclature (référentiel Chorus) garde à 0 € : vérifier qu'il
   manque bien à l'état B, puis le lister dans `nomenclature.absent_from_law`
   (369 en 2025). Un programme ainsi écarté qui porterait des crédits fait
   échouer l'import.
4. En production, après le déploiement qui apporte les fichiers (les migrations
   tournent au démarrage du service), sur le VPS :

   ```bash
   ./target/release/import-budget data/budget/lfi-<année>.json --dry-run
   ./target/release/import-budget data/budget/lfi-<année>.json
   ```

   Tout contrôle en échec arrête l'import avant la moindre écriture ; le
   rejouer remplace l'exercice à l'identique.
