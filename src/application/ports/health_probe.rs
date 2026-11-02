use async_trait::async_trait;

/// Sonde des dependances du serveur, interrogee par le health check.
///
/// Rend un etat, pas une erreur: une base injoignable est une reponse du
/// health check, pas son echec.
#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn database_reachable(&self) -> bool;
}
