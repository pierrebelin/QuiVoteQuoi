//! Où en est la loi (todo/navette/SPEC-navette.md, lot 2) : filtre d'état,
//! compteurs sur le périmètre affiché, ordre par date d'état et avancées d'une
//! semaine, lus dans un vrai Postgres.
//!
//! La CI n'a pas de base, d'où `#[ignore]`. Les migrations sont jouées sur la
//! base visée, qui doit donc être jetable.

use chrono::NaiveDate;
use quivotequoi::application::ports::dossier_repository::{
    DossierCriteria, DossierOrder, DossierRepository, LawStateCounts,
};
use quivotequoi::domain::dossier::{
    DossierOutcome, DossierUid, Initiative, LawPublication, LegislativeAct, LegislativeDossier,
    NewLegislativeDossier, Score,
};
use quivotequoi::domain::law_progress::{LawAdvance, LawStateKind};
use quivotequoi::infrastructure::persistence::pg_dossier_repository::PgDossierRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Fragment de titre propre aux dossiers de ce test : la recherche les isole
/// du reste de la base.
const MARK: &str = "NAVLOT2";
const SHUTTLE: &str = "DLR5L17NNAVLOT2S";
const ADOPTED: &str = "DLR5L17NNAVLOT2D";
const LAW: &str = "DLR5L17NNAVLOT2L";
const NONE: &str = "DLR5L17NNAVLOT2N";
const UIDS: [&str; 4] = [SHUTTLE, ADOPTED, LAW, NONE];

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn act(code: &str, on: NaiveDate, label: Option<&str>) -> LegislativeAct {
    LegislativeAct {
        date: on,
        label: code.to_string(),
        code: Some(code.to_string()),
        decision_vote: None,
        conclusion_label: label.map(str::to_string),
        conclusion_code: None,
    }
}

fn promulgated(on: NaiveDate, law_code: &str) -> DossierOutcome {
    DossierOutcome::Promulgated {
        date: on,
        publication: LawPublication {
            law_code: Some(law_code.to_string()),
            jo_date: Some(on.succ_opt().unwrap()),
            legifrance_url: None,
        },
    }
}

fn dossier(
    uid: &str,
    procedure: &str,
    acts: Vec<LegislativeAct>,
    outcome: DossierOutcome,
) -> LegislativeDossier {
    let last = acts.iter().map(|a| a.date).max().unwrap();
    LegislativeDossier::new(NewLegislativeDossier {
        uid: DossierUid::new(uid.to_string()).unwrap(),
        title: format!("Dossier {MARK} {uid}"),
        procedure: procedure.to_string(),
        legislature: 17,
        url: None,
        summary: None,
        deposit_date: None,
        last_activity_date: last,
        last_activity_label: "Décision".to_string(),
        acts,
        documents: Vec::new(),
        score: Score::new(0, 0, 0, 0).unwrap(),
        current_stage: None,
        initiators: Vec::new(),
        committee: None,
        outcome,
    })
}

/// Semaine du lundi 3 au dimanche 9 mars 2031.
fn week() -> (NaiveDate, NaiveDate) {
    (date(2031, 3, 3), date(2031, 3, 9))
}

/// S : en navette, modifié par le Sénat dans la semaine (projet de loi).
/// D : adopté conforme dans la semaine, promulgué deux semaines après.
/// L : devenu loi en 2030. N : rejeté seul dans la semaine, sans état.
fn fixture() -> Vec<LegislativeDossier> {
    let adopted = Some("adoptée");
    let unchanged = Some("adoptée sans modification");
    vec![
        dossier(
            SHUTTLE,
            "Projet de loi ordinaire",
            vec![
                act("AN1-DEBATS-DEC", date(2031, 1, 10), adopted),
                act("SN1-DEBATS-DEC", date(2031, 3, 5), Some("modifiée")),
            ],
            DossierOutcome::NoRecordedConclusion,
        ),
        dossier(
            ADOPTED,
            "Proposition de loi ordinaire",
            vec![
                act("SN1-DEBATS-DEC", date(2031, 1, 20), adopted),
                act("AN1-DEBATS-DEC", date(2031, 3, 6), unchanged),
                act("PROM-PUB", date(2031, 3, 20), None),
            ],
            promulgated(date(2031, 3, 20), "2031-300"),
        ),
        dossier(
            LAW,
            "Proposition de loi ordinaire",
            vec![
                act("SN1-DEBATS-DEC", date(2030, 11, 1), adopted),
                act("AN1-DEBATS-DEC", date(2030, 12, 1), unchanged),
                act("PROM-PUB", date(2030, 12, 15), None),
            ],
            promulgated(date(2030, 12, 15), "2030-1200"),
        ),
        dossier(
            NONE,
            "Proposition de loi ordinaire",
            vec![act("AN1-DEBATS-DEC", date(2031, 3, 4), Some("rejetée"))],
            DossierOutcome::NoRecordedConclusion,
        ),
    ]
}

async fn remove_fixture(pool: &PgPool) {
    sqlx::query("DELETE FROM legislative_acts WHERE dossier_uid = ANY($1)")
        .bind(UIDS.to_vec())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM legislative_dossiers WHERE uid = ANY($1)")
        .bind(UIDS.to_vec())
        .execute(pool)
        .await
        .unwrap();
}

async fn seeded() -> (PgPool, PgDossierRepository) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    remove_fixture(&pool).await;
    let repository = PgDossierRepository::new(pool.clone());
    repository.save_all(&fixture()).await.unwrap();
    (pool, repository)
}

fn criteria(states: Vec<LawStateKind>, initiative: Option<Initiative>) -> DossierCriteria {
    DossierCriteria {
        search: Some(MARK.to_string()),
        law_states: states,
        initiative,
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn the_list_and_the_week_read_the_law_state_from_the_database() {
    let (pool, repository) = seeded().await;

    let became_law = repository
        .find_page(
            &criteria(vec![LawStateKind::BecameLaw], None),
            DossierOrder::StateDate,
            20,
            0,
        )
        .await
        .unwrap();
    let uids: Vec<&str> = became_law
        .items
        .iter()
        .map(|l| l.dossier.uid().as_str())
        .collect();
    assert_eq!(uids, vec![ADOPTED, LAW]);
    assert_eq!(became_law.total, 2);
    let state = became_law.items[1].dossier.law_state().expect("état relu");
    assert_eq!(state.kind(), LawStateKind::BecameLaw);
    assert_eq!(state.date(), date(2030, 12, 15));
    // Compteurs : mêmes critères, sans le filtre d'état.
    assert_eq!(
        became_law.state_counts,
        LawStateCounts {
            in_shuttle: 1,
            definitively_adopted: 0,
            became_law: 2,
        }
    );

    // Le périmètre affiché suit les autres critères.
    let government = repository
        .find_page(
            &criteria(vec![LawStateKind::BecameLaw], Some(Initiative::Government)),
            DossierOrder::LastActivity,
            20,
            0,
        )
        .await
        .unwrap();
    assert_eq!(government.total, 0);
    assert_eq!(
        government.state_counts,
        LawStateCounts {
            in_shuttle: 1,
            definitively_adopted: 0,
            became_law: 0,
        }
    );

    // Date d'état décroissante ; un dossier sans état vient après.
    let all = repository
        .find_page(&criteria(vec![], None), DossierOrder::StateDate, 20, 0)
        .await
        .unwrap();
    let uids: Vec<&str> = all.items.iter().map(|l| l.dossier.uid().as_str()).collect();
    assert_eq!(uids, vec![ADOPTED, SHUTTLE, LAW, NONE]);
    assert!(all.items[3].dossier.law_state().is_none());

    // Le résumé relu sans les actes est aussi celui de `find_by_uids`.
    let listed = repository
        .find_by_uids(&[DossierUid::new(SHUTTLE.to_string()).unwrap()])
        .await
        .unwrap();
    let shuttle = listed[0].law_state().expect("état relu");
    assert_eq!(shuttle.kind(), LawStateKind::InShuttle);
    assert_eq!(
        shuttle.last_decision().map(|d| d.date),
        Some(date(2031, 3, 5))
    );

    the_texts_that_advanced_in_a_week_come_with_their_acts(&repository).await;

    remove_fixture(&pool).await;
}

/// Même jeu que la liste : un seul test, pour que deux tests parallèles ne
/// sèment ni n'effacent les mêmes lignes.
async fn the_texts_that_advanced_in_a_week_come_with_their_acts(repository: &PgDossierRepository) {
    let (monday, sunday) = week();

    let mut found: Vec<LegislativeDossier> = repository
        .find_with_decisions_between(monday, sunday)
        .await
        .unwrap()
        .into_iter()
        .filter(|d| UIDS.contains(&d.uid().as_str()))
        .collect();
    found.sort_by(|a, b| a.uid().as_str().cmp(b.uid().as_str()));
    let uids: Vec<&str> = found.iter().map(|d| d.uid().as_str()).collect();
    assert_eq!(uids, vec![ADOPTED, NONE, SHUTTLE]);
    assert_eq!(found[0].acts().len(), 3);

    let groups: Vec<(&str, Option<LawStateKind>)> = found
        .iter()
        .map(|d| {
            let advance = LawAdvance::between(d.acts(), d.outcome(), monday, sunday);
            (d.uid().as_str(), advance.map(|a| a.state.kind()))
        })
        .collect();
    // D est adopté définitivement au soir du dimanche : sa promulgation tombe
    // après. N, rejeté seul, n'a pas d'état et n'a pas avancé.
    assert_eq!(
        groups,
        vec![
            (ADOPTED, Some(LawStateKind::DefinitivelyAdopted)),
            (NONE, None),
            (SHUTTLE, Some(LawStateKind::InShuttle)),
        ]
    );
}
