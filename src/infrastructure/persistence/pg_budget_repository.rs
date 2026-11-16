use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};

use super::{db, invalid_row};
use crate::application::ports::budget_repository::{BudgetRepository, RepositoryError};
use crate::domain::budget::{
    AccountKind, AdoptionProcedure, BudgetExercise, BudgetStage, BudgetYear, Euros, LawReference,
    Mission, NewBudgetExercise, Programme, ProgrammeNumber, SourceReference,
};

pub struct PgBudgetRepository {
    pool: PgPool,
}

impl PgBudgetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BudgetRepository for PgBudgetRepository {
    async fn replace_exercise(&self, exercise: &BudgetExercise) -> Result<(), RepositoryError> {
        let year = i16::try_from(exercise.year().get()).map_err(invalid_row)?;
        let law = exercise.law();
        let source = exercise.source();

        // Une seule transaction : un échec à mi-chemin laisse l'ancien exercice
        // en place, jamais un exercice à moitié remplacé. La suppression de la
        // ligne d'exercice emporte ses missions et ses programmes (CASCADE).
        let mut tx = self.pool.begin().await.map_err(db)?;

        sqlx::query("DELETE FROM budget_exercises WHERE year = $1")
            .bind(year)
            .persistent(false)
            .execute(&mut *tx)
            .await
            .map_err(db)?;

        sqlx::query(
            "INSERT INTO budget_exercises
                (year, stage, law_number, law_date, credits_article, adoption_procedure,
                 legifrance_url, source_title, source_url, source_published_on,
                 source_downloaded_on, source_sha256)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(year)
        .bind(exercise.stage().as_str())
        .bind(&law.number)
        .bind(law.date)
        .bind(i16::try_from(law.credits_article).map_err(invalid_row)?)
        .bind(law.adoption_procedure.as_str())
        .bind(&law.legifrance_url)
        .bind(&source.title)
        .bind(&source.url)
        .bind(source.published_on)
        .bind(source.downloaded_on)
        .bind(&source.sha256)
        .persistent(false)
        .execute(&mut *tx)
        .await
        .map_err(db)?;

        for mission in exercise.missions() {
            sqlx::query(
                "INSERT INTO budget_missions (year, name, account_kind) VALUES ($1, $2, $3)",
            )
            .bind(year)
            .bind(mission.name())
            .bind(mission.account_kind().as_str())
            .persistent(false)
            .execute(&mut *tx)
            .await
            .map_err(db)?;

            for programme in mission.programmes() {
                sqlx::query(
                    "INSERT INTO budget_programmes (year, number, name, mission_name, cp_euros)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(year)
                .bind(i16::try_from(programme.number().get()).map_err(invalid_row)?)
                .bind(programme.name())
                .bind(mission.name())
                .bind(programme.cp().get())
                .persistent(false)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }

        tx.commit().await.map_err(db)?;
        Ok(())
    }

    async fn find_exercise(
        &self,
        year: BudgetYear,
    ) -> Result<Option<BudgetExercise>, RepositoryError> {
        let key = i16::try_from(year.get()).map_err(invalid_row)?;

        let Some(row) = sqlx::query(
            "SELECT stage, law_number, law_date, credits_article, adoption_procedure,
                    legifrance_url, source_title, source_url, source_published_on,
                    source_downloaded_on, source_sha256
               FROM budget_exercises
              WHERE year = $1",
        )
        .bind(key)
        .persistent(false)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        else {
            return Ok(None);
        };

        let stage: String = row.get("stage");
        let procedure: String = row.get("adoption_procedure");
        let credits_article: i16 = row.get("credits_article");
        let law = LawReference {
            number: row.get("law_number"),
            date: row.get::<NaiveDate, _>("law_date"),
            credits_article: u16::try_from(credits_article).map_err(invalid_row)?,
            adoption_procedure: AdoptionProcedure::parse(&procedure)
                .ok_or_else(|| invalid_row(format!("adoption procedure {procedure}")))?,
            legifrance_url: row.get("legifrance_url"),
        };
        let source = SourceReference {
            title: row.get("source_title"),
            url: row.get("source_url"),
            published_on: row.get("source_published_on"),
            downloaded_on: row.get("source_downloaded_on"),
            sha256: row.get("source_sha256"),
        };

        let mission_rows = sqlx::query(
            "SELECT name, account_kind FROM budget_missions WHERE year = $1 ORDER BY name",
        )
        .bind(key)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let programme_rows = sqlx::query(
            "SELECT number, name, mission_name, cp_euros
               FROM budget_programmes
              WHERE year = $1
              ORDER BY number",
        )
        .bind(key)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut programmes_of: HashMap<String, Vec<Programme>> = HashMap::new();
        for row in &programme_rows {
            let number: i16 = row.get("number");
            let programme = Programme::new(
                ProgrammeNumber::new(u16::try_from(number).map_err(invalid_row)?)
                    .map_err(invalid_row)?,
                row.get::<String, _>("name"),
                Euros::new(row.get("cp_euros")).map_err(invalid_row)?,
            )
            .map_err(invalid_row)?;
            programmes_of
                .entry(row.get("mission_name"))
                .or_default()
                .push(programme);
        }

        let mut missions = Vec::with_capacity(mission_rows.len());
        for row in &mission_rows {
            let name: String = row.get("name");
            let kind: String = row.get("account_kind");
            let kind = AccountKind::parse(&kind)
                .ok_or_else(|| invalid_row(format!("account kind {kind}")))?;
            let programmes = programmes_of.remove(&name).unwrap_or_default();
            missions.push(Mission::new(name, kind, programmes).map_err(invalid_row)?);
        }

        let exercise = BudgetExercise::new(NewBudgetExercise {
            year,
            stage: BudgetStage::parse(&stage)
                .ok_or_else(|| invalid_row(format!("budget stage {stage}")))?,
            law,
            source,
            missions,
        })
        .map_err(invalid_row)?;
        Ok(Some(exercise))
    }

    async fn available_years(&self) -> Result<Vec<BudgetYear>, RepositoryError> {
        let rows = sqlx::query("SELECT year FROM budget_exercises ORDER BY year DESC")
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;

        rows.iter()
            .map(|row| {
                let year: i16 = row.get("year");
                BudgetYear::new(u16::try_from(year).map_err(invalid_row)?).map_err(invalid_row)
            })
            .collect()
    }
}
