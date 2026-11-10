//! Lecture réelle de la liste des dossiers classée par dernier scrutin, lancée
//! à la main contre la base `DATABASE_URL`. Lecture seule : rien n'est écrit.
//!
//! Vérifie ce que les fakes ne peuvent pas : la requête SQL elle-même — ordre,
//! filtre « au moins un scrutin », décomptes adoptés / rejetés, total.
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test dossier_page_by_last_scrutin -- --ignored --nocapture
//! ```

use quivotequoi::application::ports::dossier_repository::{
    DossierCriteria, DossierOrder, DossierRepository,
};
use quivotequoi::infrastructure::config;
use quivotequoi::infrastructure::persistence::pg_dossier_repository::PgDossierRepository;

const PAGE_SIZE: i64 = 20;

#[tokio::test]
#[ignore = "lecture réelle : base à poser à la main"]
async fn voted_dossiers_come_by_last_scrutin_with_their_tally() {
    let db = config::try_connect_database()
        .await
        .expect("DATABASE_URL required");
    let repository = PgDossierRepository::new(db);
    let voted = DossierCriteria {
        with_scrutins: true,
        ..Default::default()
    };

    let page = repository
        .find_page(&voted, DossierOrder::LastScrutin, PAGE_SIZE, 0)
        .await
        .unwrap();
    let everything = repository
        .find_page(
            &DossierCriteria::default(),
            DossierOrder::LastActivity,
            1,
            0,
        )
        .await
        .unwrap();

    println!("{} dossiers votés sur {}", page.total, everything.total);
    assert!(page.total > 0 && page.total <= everything.total);
    let dates: Vec<_> = page.items.iter().map(|l| l.scrutins.last_date).collect();
    assert!(dates.iter().all(Option::is_some), "un dossier sans scrutin");
    assert!(dates.windows(2).all(|w| w[0] >= w[1]), "ordre décroissant");
    for listing in &page.items {
        let tally = listing.scrutins;
        println!(
            "{:?} {} — {} scrutins, {} adoptés, {} rejetés",
            tally.last_date,
            listing.dossier.uid().as_str(),
            tally.count,
            tally.adopted,
            tally.rejected
        );
        assert!(tally.count > 0);
        assert_eq!(tally.adopted + tally.rejected, tally.count);
    }

    let unresolved = DossierCriteria {
        with_scrutins: true,
        outcome_kinds: vec!["no_recorded_conclusion".into()],
        ..Default::default()
    };
    let narrowed = repository
        .find_page(&unresolved, DossierOrder::LastScrutin, PAGE_SIZE, 0)
        .await
        .unwrap();
    println!("{} sans conclusion à ce jour", narrowed.total);
    assert!(narrowed.total <= page.total);
    assert!(narrowed
        .items
        .iter()
        .all(|l| l.dossier.outcome().kind() == "no_recorded_conclusion"));
}
