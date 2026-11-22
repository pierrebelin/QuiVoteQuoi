//! Où en est la loi (todo/navette/SPEC-navette.md, lot 1) : l'état calculé par
//! le domaine survit à l'aller-retour en base.
//!
//! Les colonnes `law_state_*` / `last_decision_*` et la conclusion des actes ne
//! sont lues qu'à l'exécution : seul un vrai Postgres les met à l'épreuve. La
//! CI n'en a pas, d'où `#[ignore]`. Les migrations sont jouées sur la base
//! visée, qui doit donc être jetable.

use chrono::NaiveDate;
use quivotequoi::application::ports::dossier_repository::DossierRepository;
use quivotequoi::domain::dossier::{
    DossierOutcome, DossierUid, LawPublication, LegislativeAct, LegislativeDossier,
    NewLegislativeDossier, Score,
};
use quivotequoi::domain::law_progress::LawState;
use quivotequoi::infrastructure::persistence::pg_dossier_repository::PgDossierRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const SHUTTLE_UID: &str = "DLR5L17NTESTNAV1";
const LAW_UID: &str = "DLR5L17NTESTNAV2";

/// uid, état, date de l'état, chambre, sens et date de la dernière décision.
type StateRow = (
    String,
    Option<String>,
    Option<NaiveDate>,
    Option<String>,
    Option<String>,
    Option<NaiveDate>,
);

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn decision(code: &str, on: NaiveDate, label: &str) -> LegislativeAct {
    LegislativeAct {
        date: on,
        label: "Décision".to_string(),
        code: Some(code.to_string()),
        decision_vote: None,
        conclusion_label: Some(label.to_string()),
        conclusion_code: None,
    }
}

fn dossier(uid: &str, acts: Vec<LegislativeAct>, outcome: DossierOutcome) -> LegislativeDossier {
    let last = acts.iter().map(|a| a.date).max().unwrap();
    LegislativeDossier::new(NewLegislativeDossier {
        uid: DossierUid::new(uid.to_string()).unwrap(),
        title: format!("Dossier de test {uid}"),
        procedure: "Proposition de loi ordinaire".to_string(),
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

async fn remove_fixture(pool: &PgPool) {
    sqlx::query("DELETE FROM legislative_acts WHERE dossier_uid = ANY($1)")
        .bind(vec![SHUTTLE_UID, LAW_UID])
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM legislative_dossiers WHERE uid = ANY($1)")
        .bind(vec![SHUTTLE_UID, LAW_UID])
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn the_law_state_and_the_decisions_come_back_from_the_database() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    remove_fixture(&pool).await;
    let repository = PgDossierRepository::new(pool.clone());

    let in_shuttle = dossier(
        SHUTTLE_UID,
        vec![
            decision("AN1-DEBATS-DEC", date(2026, 7, 7), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 11, 20), "modifiée"),
        ],
        DossierOutcome::NoRecordedConclusion,
    );
    let became_law = dossier(
        LAW_UID,
        vec![
            decision("SN1-DEBATS-DEC", date(2026, 6, 15), "adoptée"),
            decision(
                "AN1-DEBATS-DEC",
                date(2026, 10, 12),
                "adoptée sans modification",
            ),
        ],
        DossierOutcome::Promulgated {
            date: date(2026, 10, 26),
            publication: LawPublication {
                law_code: Some("2026-992".to_string()),
                jo_date: Some(date(2026, 10, 27)),
                legifrance_url: None,
            },
        },
    );
    repository
        .save_all(&[in_shuttle, became_law])
        .await
        .unwrap();

    let rows: Vec<StateRow> =
        sqlx::query_as(
            "SELECT uid, law_state_kind, law_state_date, last_decision_chamber, last_decision_sense, last_decision_date
             FROM legislative_dossiers WHERE uid = ANY($1) ORDER BY uid",
        )
        .bind(vec![SHUTTLE_UID, LAW_UID])
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let shuttle = &rows[0];
    assert_eq!(shuttle.0, SHUTTLE_UID);
    assert_eq!(shuttle.1.as_deref(), Some("in_shuttle"));
    assert_eq!(shuttle.2, Some(date(2026, 11, 20)));
    assert!(shuttle.3.is_some(), "chambre de la dernière décision");
    assert!(shuttle.4.is_some(), "sens de la dernière décision");
    assert_eq!(shuttle.5, Some(date(2026, 11, 20)));
    let law = &rows[1];
    assert_eq!(law.1.as_deref(), Some("became_law"));
    assert_eq!(law.2, Some(date(2026, 10, 26)));

    let states = repository.load_states().await.unwrap();
    assert!(states[SHUTTLE_UID].law_progress_computed);
    assert!(states[LAW_UID].law_progress_computed);

    // Les jalons du détail se recalculent depuis les actes relus.
    let reread = repository
        .find_by_uid(&DossierUid::new(SHUTTLE_UID.to_string()).unwrap())
        .await
        .unwrap()
        .expect("dossier relu");
    assert_eq!(
        reread.acts()[1].conclusion_label.as_deref(),
        Some("modifiée")
    );
    let progress = reread.law_progress();
    assert!(matches!(progress.state(), Some(LawState::InShuttle { .. })));
    assert_eq!(progress.milestones().len(), 4);

    remove_fixture(&pool).await;
}
