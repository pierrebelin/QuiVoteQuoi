use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::PgPool;

use super::db;
use crate::application::ports::actor_repository::{
    ActorRepository, RegistrySummary, RepositoryError,
};
use crate::domain::actor::{
    Actor, ActorDirectory, ActorRegistry, ActorRole, ActorUid, GroupMembership, GroupUid,
    MembershipPeriod, MembershipQuality, ParliamentaryGroup,
};

/// Taille des lots d'ecriture. Le referentiel complet tient en quelques lots:
/// il est ecrit par insertion groupee, pas ligne a ligne.
const BATCH_SIZE: usize = 500;

pub struct PgActorRepository {
    pool: PgPool,
}

impl PgActorRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ActorRepository for PgActorRepository {
    async fn save_registry(
        &self,
        registry: &ActorRegistry,
    ) -> Result<RegistrySummary, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db)?;

        for chunk in registry.groups.chunks(BATCH_SIZE) {
            let uids: Vec<&str> = chunk.iter().map(|g| g.uid().as_str()).collect();
            let legislatures: Vec<i16> = chunk.iter().map(|g| g.legislature() as i16).collect();
            let labels: Vec<&str> = chunk.iter().map(|g| g.label()).collect();
            let abbrevs: Vec<&str> = chunk.iter().map(|g| g.abbrev()).collect();
            let colors: Vec<Option<&str>> = chunk.iter().map(|g| g.color()).collect();
            let starts: Vec<Option<NaiveDate>> = chunk.iter().map(|g| g.start_date()).collect();
            let ends: Vec<Option<NaiveDate>> = chunk.iter().map(|g| g.end_date()).collect();

            sqlx::query(
                "INSERT INTO parliamentary_groups (uid, legislature, label, abbrev, color, start_date, end_date)
                 SELECT * FROM UNNEST($1::text[], $2::smallint[], $3::text[], $4::text[], $5::text[], $6::date[], $7::date[])
                 ON CONFLICT (uid) DO UPDATE SET
                    legislature = EXCLUDED.legislature,
                    label = EXCLUDED.label,
                    abbrev = EXCLUDED.abbrev,
                    color = EXCLUDED.color,
                    start_date = EXCLUDED.start_date,
                    end_date = EXCLUDED.end_date,
                    updated_at = NOW()",
            )
            .bind(&uids)
            .bind(&legislatures)
            .bind(&labels)
            .bind(&abbrevs)
            .bind(&colors)
            .bind(&starts)
            .bind(&ends)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        for chunk in registry.actors.chunks(BATCH_SIZE) {
            let uids: Vec<&str> = chunk.iter().map(|a| a.uid().as_str()).collect();
            let civilities: Vec<Option<&str>> = chunk.iter().map(|a| a.civility()).collect();
            let first_names: Vec<&str> = chunk.iter().map(|a| a.first_name()).collect();
            let last_names: Vec<&str> = chunk.iter().map(|a| a.last_name()).collect();
            let roles: Vec<&str> = chunk.iter().map(|a| a.role().as_str()).collect();

            sqlx::query(
                "INSERT INTO actors (uid, civility, first_name, last_name, role)
                 SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::text[])
                 ON CONFLICT (uid) DO UPDATE SET
                    civility = EXCLUDED.civility,
                    first_name = EXCLUDED.first_name,
                    last_name = EXCLUDED.last_name,
                    role = EXCLUDED.role,
                    updated_at = NOW()",
            )
            .bind(&uids)
            .bind(&civilities)
            .bind(&first_names)
            .bind(&last_names)
            .bind(&roles)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        // Aucune suppression: une appartenance close reste consultable, elle
        // porte les actes de sa periode.
        for chunk in registry.memberships.chunks(BATCH_SIZE) {
            let source_uids: Vec<&str> = chunk.iter().map(|m| m.source_uid()).collect();
            let actor_uids: Vec<&str> = chunk.iter().map(|m| m.actor_uid().as_str()).collect();
            let group_uids: Vec<&str> = chunk.iter().map(|m| m.group_uid().as_str()).collect();
            let legislatures: Vec<i16> = chunk.iter().map(|m| m.legislature() as i16).collect();
            let starts: Vec<NaiveDate> = chunk.iter().map(|m| m.period().start()).collect();
            let ends: Vec<Option<NaiveDate>> = chunk.iter().map(|m| m.period().end()).collect();
            let qualities: Vec<&str> = chunk.iter().map(|m| m.quality().as_str()).collect();

            sqlx::query(
                "INSERT INTO group_memberships (source_uid, actor_uid, group_uid, legislature, start_date, end_date, quality)
                 SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::smallint[], $5::date[], $6::date[], $7::text[])
                 ON CONFLICT (source_uid) DO UPDATE SET
                    actor_uid = EXCLUDED.actor_uid,
                    group_uid = EXCLUDED.group_uid,
                    legislature = EXCLUDED.legislature,
                    start_date = EXCLUDED.start_date,
                    end_date = EXCLUDED.end_date,
                    quality = EXCLUDED.quality,
                    updated_at = NOW()",
            )
            .bind(&source_uids)
            .bind(&actor_uids)
            .bind(&group_uids)
            .bind(&legislatures)
            .bind(&starts)
            .bind(&ends)
            .bind(&qualities)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        tx.commit().await.map_err(db)?;

        Ok(RegistrySummary {
            actors: registry.actors.len(),
            groups: registry.groups.len(),
            memberships: registry.memberships.len(),
        })
    }

    async fn load_directory_for(
        &self,
        actor_uids: &[ActorUid],
    ) -> Result<ActorDirectory, RepositoryError> {
        if actor_uids.is_empty() {
            return Ok(ActorDirectory::new(vec![], vec![], vec![]));
        }

        let uids: Vec<String> = actor_uids.iter().map(|u| u.as_str().to_string()).collect();

        let actor_rows = sqlx::query_as::<_, ActorRow>(
            "SELECT uid, civility, first_name, last_name, role FROM actors WHERE uid = ANY($1)",
        )
        .bind(&uids)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        // Toutes les appartenances de ces acteurs, closes comprises: le groupe
        // se lit a la date de l'acte, pas a aujourd'hui (RM-01).
        let membership_rows = sqlx::query_as::<_, MembershipRow>(
            "SELECT source_uid, actor_uid, group_uid, legislature, start_date, end_date, quality
             FROM group_memberships
             WHERE actor_uid = ANY($1)
             ORDER BY start_date",
        )
        .bind(&uids)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let group_rows = sqlx::query_as::<_, GroupRow>(
            "SELECT uid, legislature, label, abbrev, color, start_date, end_date
             FROM parliamentary_groups",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(ActorDirectory::new(
            actor_rows
                .into_iter()
                .filter_map(ActorRow::into_actor)
                .collect(),
            group_rows
                .into_iter()
                .filter_map(GroupRow::into_group)
                .collect(),
            membership_rows
                .into_iter()
                .filter_map(MembershipRow::into_membership)
                .collect(),
        ))
    }
}

#[derive(sqlx::FromRow)]
struct ActorRow {
    uid: String,
    civility: Option<String>,
    first_name: String,
    last_name: String,
    role: String,
}

impl ActorRow {
    fn into_actor(self) -> Option<Actor> {
        let uid = ActorUid::new(self.uid).ok()?;
        let role = ActorRole::parse(&self.role).unwrap_or(ActorRole::Other);
        Actor::new(uid, self.civility, self.first_name, self.last_name, role).ok()
    }
}

#[derive(sqlx::FromRow)]
struct GroupRow {
    uid: String,
    legislature: i16,
    label: String,
    abbrev: String,
    color: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

impl GroupRow {
    fn into_group(self) -> Option<ParliamentaryGroup> {
        let uid = GroupUid::new(self.uid).ok()?;
        ParliamentaryGroup::new(
            uid,
            self.legislature as u16,
            self.label,
            self.abbrev,
            self.color,
            self.start_date,
            self.end_date,
        )
        .ok()
    }
}

#[derive(sqlx::FromRow)]
struct MembershipRow {
    source_uid: String,
    actor_uid: String,
    group_uid: String,
    legislature: i16,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    quality: String,
}

impl MembershipRow {
    fn into_membership(self) -> Option<GroupMembership> {
        Some(GroupMembership::new(
            self.source_uid,
            ActorUid::new(self.actor_uid).ok()?,
            GroupUid::new(self.group_uid).ok()?,
            self.legislature as u16,
            MembershipPeriod::new(self.start_date, self.end_date).ok()?,
            MembershipQuality::new(self.quality).ok()?,
        ))
    }
}
