use crate::application::ports::health_probe::HealthProbe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    pub database_reachable: bool,
}

pub struct CheckHealth<'a> {
    probe: &'a dyn HealthProbe,
}

impl<'a> CheckHealth<'a> {
    pub fn new(probe: &'a dyn HealthProbe) -> Self {
        Self { probe }
    }

    pub async fn execute(&self) -> HealthReport {
        HealthReport {
            database_reachable: self.probe.database_reachable().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;

    struct FixedHealthProbe {
        database_reachable: bool,
    }

    #[async_trait]
    impl HealthProbe for FixedHealthProbe {
        async fn database_reachable(&self) -> bool {
            self.database_reachable
        }
    }

    #[tokio::test]
    async fn an_unreachable_database_is_reported_not_raised() {
        let probe = FixedHealthProbe {
            database_reachable: false,
        };

        let report = CheckHealth::new(&probe).execute().await;

        assert!(!report.database_reachable);
    }

    #[tokio::test]
    async fn a_reachable_database_is_reported() {
        let probe = FixedHealthProbe {
            database_reachable: true,
        };

        let report = CheckHealth::new(&probe).execute().await;

        assert!(report.database_reachable);
    }
}
