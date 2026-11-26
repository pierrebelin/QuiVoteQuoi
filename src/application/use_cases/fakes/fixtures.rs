//! Fixtures partagees des tests de use case: des valeurs de domaine pretes a
//! l'emploi, que plusieurs use cases construisent a l'identique.

use chrono::NaiveDate;

use crate::application::ports::group_repository::GroupRecord;
use crate::application::use_cases::fakes::group_repository::InMemoryGroupRepository;
use crate::domain::dossier::{
    DossierOutcome, DossierUid, LegislativeAct, LegislativeDossier, NewLegislativeDossier, Score,
};

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

pub fn today() -> NaiveDate {
    date(2026, 8, 7)
}

pub fn record(uid: &str, abbrev: &str, members: i64, scrutins: i64) -> GroupRecord {
    GroupRecord {
        uid: uid.to_string(),
        legislature: 17,
        label: format!("Groupe {abbrev}"),
        abbrev: abbrev.to_string(),
        color: Some("#3367A7".to_string()),
        start_date: Some(date(2024, 7, 18)),
        end_date: None,
        reference_date: today(),
        member_count: members,
        scrutin_count: scrutins,
        first_scrutin_date: Some(date(2024, 10, 8)),
        last_scrutin_date: Some(date(2026, 7, 21)),
    }
}

/// Le cas UDR devenu UDDPLR: deux identifiants, deux periodes disjointes
/// qui partitionnent exactement la legislature (H3).
pub fn renamed_repository() -> InMemoryGroupRepository {
    let mut former = record("PO847173", "UDR", 16, 3_053);
    former.end_date = Some(date(2025, 7, 10));
    former.reference_date = date(2025, 7, 10);
    former.first_scrutin_date = Some(date(2024, 10, 8));
    former.last_scrutin_date = Some(date(2025, 7, 10));

    let mut current = record("PO872880", "UDDPLR", 16, 5_381);
    current.start_date = Some(date(2025, 9, 8));
    current.first_scrutin_date = Some(date(2025, 9, 8));

    let repository = InMemoryGroupRepository::default();
    *repository.records.lock().unwrap() =
        vec![record("PO845401", "RN", 123, 8_434), former, current];
    repository
}

/// Elements d'un dossier de la source, avant construction: un test qui veut
/// une variante (sort, acte supplementaire) les ajuste ici.
pub fn dossier_parts(
    uid: &str,
    deposit_date: Option<NaiveDate>,
    last_activity: NaiveDate,
) -> NewLegislativeDossier {
    NewLegislativeDossier {
        uid: DossierUid::new(uid.into()).unwrap(),
        title: format!("Loi {uid}"),
        procedure: "Proposition de loi".into(),
        legislature: 17,
        url: None,
        summary: None,
        deposit_date,
        last_activity_date: last_activity,
        last_activity_label: "D\u{00e9}p\u{00f4}t".into(),
        acts: vec![LegislativeAct {
            date: last_activity,
            label: "D\u{00e9}p\u{00f4}t".into(),
            code: Some("AN1-DEPOT".into()),
            decision_vote: None,
            conclusion_label: None,
            conclusion_code: None,
        }],
        documents: vec![],
        score: Score::new(2, 4, 2, 23).unwrap(),
        current_stage: None,
        initiators: vec![],
        committee: None,
        outcome: DossierOutcome::NoRecordedConclusion,
    }
}

pub fn dossier(
    uid: &str,
    deposit_date: Option<NaiveDate>,
    last_activity: NaiveDate,
) -> LegislativeDossier {
    LegislativeDossier::new(dossier_parts(uid, deposit_date, last_activity))
}
