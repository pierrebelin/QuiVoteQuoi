//! Telechargement des archives de data.assemblee-nationale.fr.
//!
//! Les trois sources (dossiers, scrutins, referentiel des acteurs) suivent le
//! meme schema: un fichier ZIP complet, republie tel quel, sans sous-ensemble a
//! demander ni flux incremental (RM-01, RM-05). Elles partagent donc le meme
//! client: une identite declaree, un cache en memoire, et surtout une
//! revalidation conditionnelle plutot qu'un retelechargement systematique.

use bytes::{Bytes, BytesMut};

use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::header::{
    ACCEPT_RANGES, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED, RANGE,
};
use reqwest::StatusCode;

use crate::application::ports::SourceError;

/// Duree pendant laquelle l'archive en memoire est servie sans meme demander a
/// la source si elle a change. Couvre le cas d'un rafraichissement relance a la
/// main juste apres un autre.
const CACHE_TTL: Duration = Duration::from_secs(3600);

/// Identite du client aupres de la source.
///
/// Un client anonyme est le premier profil qu'un administrateur filtre quand il
/// resserre les vannes. Se nommer, avec une adresse ou nous joindre, c'est la
/// difference entre « bot inconnu » et « projet qu'on previent avant de
/// couper ».
const USER_AGENT: &str = concat!(
    "QuiVoteQuoi/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/pierrebelin/QuiVoteQuoi)"
);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Delai d'**inactivite**, et non duree totale de la requete.
///
/// Un plafond sur la duree totale est une guillotine: il ne distingue pas un
/// transfert lent qui progresse d'un transfert bloque. L'archive des
/// amendements a depasse les dix minutes qui suffisaient aux trois autres, et
/// reqwest a rendu l'abandon du flux sous la forme « error decoding response
/// body » — un message qui ne dit rien de la cause. Ici, seul un silence de la
/// source interrompt le telechargement, quelle que soit la taille de l'archive.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

/// Intervalle entre deux lignes de progression. Assez espace pour ne pas noyer
/// le journal, assez frequent pour qu'un telechargement d'un quart d'heure
/// montre qu'il avance.
const PROGRESS_STEP: u64 = 50 * 1024 * 1024;

/// Reprises tentees avant d'abandonner un telechargement.
///
/// La source a montre qu'elle coupe les transferts longs. Reprendre a l'octet
/// ou l'on s'est arrete coute une requete; tout recommencer coute l'archive
/// entiere. Trois tentatives suffisent a passer un incident sans masquer une
/// panne durable.
const MAX_RESUME_ATTEMPTS: usize = 3;

/// Ce que la source a renvoye avec l'archive et qui permet de lui redemander
/// « seulement si ca a change ».
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Validators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl Validators {
    fn is_empty(&self) -> bool {
        self.etag.is_none() && self.last_modified.is_none()
    }
}

/// Ce qu'il y a a faire avant de servir l'archive.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Decision {
    /// Copie recente: la servir sans rien demander.
    ServeCached,
    /// Copie datee mais identifiable: demander a la source si elle a change.
    Revalidate(Validators),
    /// Rien en cache, ou une copie que la source ne sait pas identifier.
    Download,
}

struct CachedArchive {
    /// `Bytes` et non `Vec<u8>`: le compteur de references rend la mise en
    /// cache et le service depuis le cache gratuits. Avec un `Vec`, une archive
    /// de plusieurs centaines de mega-octets etait recopiee a chaque passage.
    data: Bytes,
    fetched_at: Instant,
    validators: Validators,
}

pub struct ArchiveFetcher {
    http: reqwest::Client,
    url: &'static str,
    /// Nom court pour les journaux: « dossiers », « scrutins »...
    label: &'static str,
    cache: Mutex<Option<CachedArchive>>,
}

impl ArchiveFetcher {
    pub fn new(url: &'static str, label: &'static str) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            // HTTP/1.1 impose, et c'est delibere. Le telechargement de
            // l'archive des amendements s'interrompait a 16 572 066 octets sur
            // 296 735 207 — a deux cents kilo-octets de 16 Mio, la signature
            // d'un blocage de la fenetre de controle de flux HTTP/2. Le
            // multiplexage n'apporte rien a une requete unique sur un fichier
            // statique; il n'apporte ici qu'une facon de plus d'echouer.
            .http1_only()
            // Repris tel quel du code precedent. A reexaminer: desactiver la
            // verification du certificat n'a de raison d'etre que si la chaine
            // de la source est cassee, ce qui se verifie.
            .danger_accept_invalid_certs(true)
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            url,
            label,
            cache: Mutex::new(None),
        }
    }

    fn decide(cached: Option<&CachedArchive>, now: Instant) -> Decision {
        let Some(cached) = cached else {
            return Decision::Download;
        };

        if now.duration_since(cached.fetched_at) < CACHE_TTL {
            return Decision::ServeCached;
        }

        if cached.validators.is_empty() {
            return Decision::Download;
        }

        Decision::Revalidate(cached.validators.clone())
    }

    /// Identite de l'archive actuellement en cache, telle que la source la
    /// publie: `ETag` de preference, `Last-Modified` a defaut.
    ///
    /// Sert a reconnaitre une archive deja ingeree sans la reparcourir. On
    /// prefere le validateur publie a une empreinte calculee: hacher plusieurs
    /// centaines de mega-octets pour retrouver ce que la source annonce
    /// gratuitement dans ses en-tetes serait du travail pour rien.
    ///
    /// `None` quand rien n'est en cache, ou quand la source ne publie aucun
    /// validateur: l'appelant doit alors ingerer sans pouvoir se comparer.
    pub fn archive_id(&self) -> Option<String> {
        let cache = self.cache.lock().unwrap();
        let cached = cache.as_ref()?;
        cached
            .validators
            .etag
            .clone()
            .or_else(|| cached.validators.last_modified.clone())
    }

    /// Rend l'archive, en la retelechargeant seulement si la source dit qu'elle
    /// a change.
    ///
    /// Le processus qui appelle est le serveur lui-meme, vivant plusieurs
    /// jours: le cache et ses validateurs traversent donc les passages
    /// successifs du rafraichissement periodique. Sur une archive republiee une
    /// fois par jour et relue toutes les deux heures, onze passages sur douze
    /// se resolvent en un `304 Not Modified` de quelques octets — et evitent au
    /// passage de reparser un ZIP identique.
    pub async fn fetch(&self) -> Result<Bytes, SourceError> {
        let decision = {
            let cache = self.cache.lock().unwrap();
            Self::decide(cache.as_ref(), Instant::now())
        };

        let validators = match decision {
            Decision::ServeCached => {
                tracing::debug!("Using cached {} archive", self.label);
                let cache = self.cache.lock().unwrap();
                // Seule la tache de rafraichissement ecrit ici, et jamais deux
                // en parallele: la copie decidee juste au-dessus est encore la.
                if let Some(cached) = cache.as_ref() {
                    return Ok(cached.data.clone());
                }
                Validators::default()
            }
            Decision::Revalidate(validators) => validators,
            Decision::Download => Validators::default(),
        };

        let mut request = self.http.get(self.url);
        if let Some(etag) = &validators.etag {
            request = request.header(IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = &validators.last_modified {
            request = request.header(IF_MODIFIED_SINCE, last_modified);
        }

        if validators.is_empty() {
            tracing::info!("Downloading {}", self.url);
        } else {
            tracing::info!("Revalidating {}", self.url);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SourceError::Download(e.to_string()))?;

        if response.status() == StatusCode::NOT_MODIFIED {
            let cache = self.cache.lock().unwrap();
            let Some(cached) = cache.as_ref() else {
                // La source repond « inchangee » a une question que seule une
                // copie en cache permet de poser. Sans elle, il n'y a rien a
                // servir: le dire plutot que de rendre du vide.
                return Err(SourceError::Download(format!(
                    "{} archive: 304 Not Modified without a cached copy",
                    self.label
                )));
            };
            tracing::info!(
                "{} archive unchanged ({} bytes served from cache)",
                self.label,
                cached.data.len()
            );
            return Ok(cached.data.clone());
        }

        if !response.status().is_success() {
            return Err(SourceError::Download(format!("HTTP {}", response.status())));
        }

        let validators = Validators {
            etag: header_value(&response, ETAG),
            last_modified: header_value(&response, LAST_MODIFIED),
        };
        if validators.is_empty() {
            // Sans validateur, chaque passage repartira sur un telechargement
            // complet. Ce n'est pas une panne, c'est un cout: le signaler.
            tracing::warn!(
                "{} archive carries neither ETag nor Last-Modified: every pass will download it in full",
                self.label
            );
        }

        let data = self.download_body(response).await?;

        {
            let mut cache = self.cache.lock().unwrap();
            *cache = Some(CachedArchive {
                data: data.clone(),
                fetched_at: Instant::now(),
                validators,
            });
        }

        Ok(data)
    }

    /// Lit le corps morceau par morceau plutot qu'en un bloc.
    ///
    /// `Response::bytes()` ne rend que deux issues: l'archive entiere, ou une
    /// erreur qui ne dit pas ou elle s'est arretee. Sur une archive de plusieurs
    /// centaines de mega-octets, la difference entre « la source a refuse » et
    /// « le transfert a ete coupe a 60 % » est justement ce qu'il faut savoir.
    ///
    /// La taille annoncee est journalisee avant de commencer: c'est la seule
    /// mesure de volumetrie que le projet ait sur cette archive
    /// (todo/SPEC-amendements.md §6, H1).
    async fn download_body(&self, response: reqwest::Response) -> Result<Bytes, SourceError> {
        let announced = response.content_length();
        // `Accept-Ranges: bytes` est la promesse de la source qu'elle sait
        // reprendre. Sans elle, une coupure impose de tout recommencer.
        let resumable =
            header_value(&response, ACCEPT_RANGES).is_some_and(|v| v.eq_ignore_ascii_case("bytes"));

        match announced {
            Some(total) => tracing::info!(
                "{}: downloading {} bytes announced ({})",
                self.label,
                total,
                if resumable {
                    "resumable"
                } else {
                    "not resumable"
                }
            ),
            None => tracing::info!("{}: downloading, size not announced", self.label),
        }

        // La capacite annoncee evite de reallouer en cours de route, ce qui
        // doublerait transitoirement la memoire tenue.
        let mut buffer = BytesMut::with_capacity(announced.unwrap_or(0) as usize);
        let mut response = response;
        let mut attempts = 0usize;

        loop {
            match self.read_into(&mut buffer, response).await {
                Ok(()) => break,
                Err(interruption) => {
                    // Une reprise n'a de sens que si la source la sait, si elle
                    // a deja livre quelque chose, et si l'on n'a pas deja
                    // insiste. Sinon on rend l'erreur telle quelle.
                    if !resumable || buffer.is_empty() || attempts >= MAX_RESUME_ATTEMPTS {
                        return Err(interruption);
                    }
                    attempts += 1;
                    tracing::warn!(
                        "{} — reprise {}/{} a partir de l'octet {}",
                        interruption,
                        attempts,
                        MAX_RESUME_ATTEMPTS,
                        buffer.len()
                    );
                    response = self.request_from(buffer.len() as u64).await?;
                }
            }
        }

        // Une archive tronquee sans erreur reseau existe: la source ferme la
        // connexion proprement au milieu. Sans ce controle, le ZIP part au
        // parseur et echoue plus loin, sur un message qui ne designe plus la
        // cause.
        if let Some(total) = announced {
            if buffer.len() as u64 != total {
                return Err(SourceError::Download(format!(
                    "{}: truncated body, {} bytes received of {} announced",
                    self.label,
                    buffer.len(),
                    total
                )));
            }
        }

        tracing::info!("Downloaded {} bytes ({})", buffer.len(), self.label);
        Ok(buffer.freeze())
    }

    /// Verse le corps d'une reponse dans le tampon, jusqu'a sa fin ou jusqu'a
    /// l'interruption. Le tampon garde ce qui est passe: c'est lui qui rend la
    /// reprise possible.
    async fn read_into(
        &self,
        buffer: &mut BytesMut,
        mut response: reqwest::Response,
    ) -> Result<(), SourceError> {
        let mut next_report = (buffer.len() as u64 / PROGRESS_STEP + 1) * PROGRESS_STEP;

        loop {
            let chunk = response.chunk().await.map_err(|e| {
                SourceError::Download(format!(
                    "{}: transfer interrupted after {} bytes: {}",
                    self.label,
                    buffer.len(),
                    causes(&e)
                ))
            })?;

            let Some(chunk) = chunk else { return Ok(()) };
            buffer.extend_from_slice(&chunk);

            if buffer.len() as u64 >= next_report {
                tracing::info!("{}: {} bytes received", self.label, buffer.len());
                next_report += PROGRESS_STEP;
            }
        }
    }

    /// Redemande l'archive a partir d'un octet donne.
    ///
    /// La source doit repondre `206 Partial Content`. Un `200` signifierait
    /// qu'elle ignore l'en-tete et renvoie tout depuis le debut: concatener
    /// produirait une archive corrompue, donc on refuse plutot que d'abimer.
    async fn request_from(&self, offset: u64) -> Result<reqwest::Response, SourceError> {
        let response = self
            .http
            .get(self.url)
            .header(RANGE, format!("bytes={offset}-"))
            .send()
            .await
            .map_err(|e| {
                SourceError::Download(format!(
                    "{}: resume request failed: {}",
                    self.label,
                    causes(&e)
                ))
            })?;

        if response.status() != StatusCode::PARTIAL_CONTENT {
            return Err(SourceError::Download(format!(
                "{}: resume refused, HTTP {} instead of 206",
                self.label,
                response.status()
            )));
        }

        Ok(response)
    }
}

/// Message d'une erreur, suivi de sa chaine de causes.
///
/// `reqwest::Error` n'affiche que son propre libelle — « error decoding
/// response body » — et laisse dans `source()` ce qui s'est reellement passe:
/// connexion coupee, flux reinitialise, delai expire. Sans derouler la chaine,
/// le journal nomme le symptome et tait la cause.
fn causes(error: &dyn std::error::Error) -> String {
    let mut out = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        out.push_str(" <- ");
        out.push_str(&cause.to_string());
        source = cause.source();
    }
    out
}

fn header_value(response: &reqwest::Response, name: reqwest::header::HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Erreur a deux etages, comme celles de reqwest: le libelle de surface ne
    /// dit rien, la cause est en dessous.
    #[derive(Debug)]
    struct Layered {
        message: &'static str,
        source: Option<Box<Layered>>,
    }

    impl std::fmt::Display for Layered {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.message)
        }
    }

    impl std::error::Error for Layered {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.source
                .as_deref()
                .map(|s| s as &(dyn std::error::Error + 'static))
        }
    }

    #[test]
    fn the_cause_chain_is_unrolled_not_swallowed() {
        let error = Layered {
            message: "error decoding response body",
            source: Some(Box::new(Layered {
                message: "connection reset by peer",
                source: None,
            })),
        };

        assert_eq!(
            causes(&error),
            "error decoding response body <- connection reset by peer"
        );
    }

    /// Une erreur sans cause ne gagne pas de decoration.
    #[test]
    fn a_lone_error_reads_as_itself() {
        let error = Layered {
            message: "HTTP 503",
            source: None,
        };
        assert_eq!(causes(&error), "HTTP 503");
    }

    fn cached(age: Duration, validators: Validators) -> CachedArchive {
        CachedArchive {
            data: Bytes::from_static(&[1, 2, 3]),
            fetched_at: Instant::now() - age,
            validators,
        }
    }

    fn etag(value: &str) -> Validators {
        Validators {
            etag: Some(value.to_string()),
            last_modified: None,
        }
    }

    #[test]
    fn nothing_cached_means_downloading() {
        assert_eq!(
            ArchiveFetcher::decide(None, Instant::now()),
            Decision::Download
        );
    }

    #[test]
    fn a_recent_copy_is_served_without_asking_anything() {
        let cached = cached(Duration::from_secs(60), etag("\"abc\""));

        assert_eq!(
            ArchiveFetcher::decide(Some(&cached), Instant::now()),
            Decision::ServeCached
        );
    }

    /// Le cas du rafraichissement periodique: la copie a deux heures, la source
    /// est interrogee, mais avec son validateur — pas un telechargement sec.
    #[test]
    fn a_dated_copy_is_revalidated_not_downloaded() {
        let cached = cached(Duration::from_secs(7200), etag("\"abc\""));

        assert_eq!(
            ArchiveFetcher::decide(Some(&cached), Instant::now()),
            Decision::Revalidate(etag("\"abc\""))
        );
    }

    #[test]
    fn last_modified_alone_is_enough_to_revalidate() {
        let validators = Validators {
            etag: None,
            last_modified: Some("Wed, 06 Aug 2026 09:00:00 GMT".to_string()),
        };
        let cached = cached(Duration::from_secs(7200), validators.clone());

        assert_eq!(
            ArchiveFetcher::decide(Some(&cached), Instant::now()),
            Decision::Revalidate(validators)
        );
    }

    /// Sans validateur, il n'y a rien a demander conditionnellement: la seule
    /// facon de savoir si l'archive a change est de la reprendre.
    #[test]
    fn a_dated_copy_without_validator_is_downloaded_again() {
        let cached = cached(Duration::from_secs(7200), Validators::default());

        assert_eq!(
            ArchiveFetcher::decide(Some(&cached), Instant::now()),
            Decision::Download
        );
    }

    #[test]
    fn the_client_names_itself_and_says_where_to_reach_us() {
        assert!(USER_AGENT.starts_with("QuiVoteQuoi/"));
        assert!(USER_AGENT.contains("https://github.com/pierrebelin/QuiVoteQuoi"));
    }
}
