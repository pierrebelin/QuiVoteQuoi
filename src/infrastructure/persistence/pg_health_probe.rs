use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::ports::health_probe::HealthProbe;

pub struct PgHealthProbe {
    pool: PgPool,
}

impl PgHealthProbe {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HealthProbe for PgHealthProbe {
    async fn database_reachable(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.pool).await.is_ok()
    }
}
