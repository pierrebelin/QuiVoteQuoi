//! Affiche le jeton d'administration du jour.
//!
//! Le secret est lu dans l'environnement, jamais passe en argument : `argv`
//! est lisible par tout utilisateur de la machine via `ps`, l'environnement
//! d'un processus ne l'est que par son proprietaire et par root.
//!
//! ```bash
//! # sur le VPS
//! ~/app/deploy/bin/admin-token.sh
//! # depuis le poste local
//! ssh hemicycle@<IP_DU_VPS> '~/app/deploy/bin/admin-token.sh'
//! ```

use chrono::Utc;
use quivotequoi::infrastructure::security::AdminTokenSecret;

fn main() -> std::process::ExitCode {
    dotenvy::dotenv().ok();

    let Ok(raw) = std::env::var("ADMIN_TOKEN_SECRET") else {
        eprintln!("ADMIN_TOKEN_SECRET absent de l'environnement.");
        return std::process::ExitCode::FAILURE;
    };

    match AdminTokenSecret::new(raw) {
        Ok(secret) => {
            println!("{}", secret.token_for(Utc::now().date_naive()));
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
