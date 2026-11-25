//! Lecture réelle des familles d'un dossier, lancée à la main contre la base
//! `DATABASE_URL`. Lecture seule : rien n'est écrit.
//!
//! Vérifie ce que les fakes ne peuvent pas : la requête SQL elle-même. Un
//! dossier dont plusieurs textes (dépôt, navette, CMP) portent la même famille,
//! classés à des dates différentes, ne la montre qu'une fois.
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test dossier_families_read_back -- --ignored --nocapture
//! ```

use std::collections::HashSet;

use quivotequoi::application::ports::dossier_repository::{
    DossierCriteria, DossierOrder, DossierRepository,
};
use quivotequoi::application::ports::theme_repository::ThemeRepository;
use quivotequoi::infrastructure::config;
use quivotequoi::infrastructure::persistence::pg_dossier_repository::PgDossierRepository;
use quivotequoi::infrastructure::persistence::pg_theme_repository::PgThemeRepository;

const PAGE_SIZE: i64 = 100;

#[tokio::test]
#[ignore = "lecture réelle : base à poser à la main"]
async fn a_dossier_shows_each_family_once() {
    let db = config::try_connect_database()
        .await
        .expect("DATABASE_URL required");
    let dossiers = PgDossierRepository::new(db.clone());
    let themes = PgThemeRepository::new(db);
    let voted = DossierCriteria {
        with_scrutins: true,
        ..Default::default()
    };

    let page = dossiers
        .find_page(&voted, DossierOrder::LastScrutin, PAGE_SIZE, 0)
        .await
        .unwrap();

    assert!(!page.items.is_empty(), "aucun dossier voté dans la base");
    for listing in &page.items {
        let uid = listing.dossier.uid().as_str();
        let families = themes.families_of_dossier(uid).await.unwrap();
        let codes: Vec<&str> = families.iter().map(|f| f.family.as_str()).collect();
        let distinct: HashSet<&str> = codes.iter().copied().collect();
        assert_eq!(codes.len(), distinct.len(), "{uid} : familles en double {codes:?}");
    }
}
