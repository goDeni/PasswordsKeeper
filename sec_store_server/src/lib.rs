pub mod api;

use std::collections::HashMap;
use std::io::{BufReader, Cursor};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use axum::{http::StatusCode, Router};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use sec_store::repository::file::{NamedFileRepositories, RecordsFileRepository};
use sec_store::repository::{CreateRepositoryError, RepositoryOpenError, UpdateRecordError};
use tokio::sync::{Mutex, RwLock};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ServerConfigPaths {
    pub bind_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub server_cert_pem: PathBuf,
    pub server_key_pem: PathBuf,
    pub client_ca_cert_pem: PathBuf,
}

#[derive(Clone)]
pub struct AppState {
    repositories: NamedFileRepositories,
    sessions: Arc<RwLock<HashMap<String, Arc<Mutex<SessionState>>>>>,
}

#[derive(Debug)]
pub(crate) struct SessionState {
    pub(crate) repository: RecordsFileRepository,
    pub(crate) persisted_snapshot: Vec<u8>,
}

impl AppState {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(&data_dir)
            .await
            .with_context(|| format!("Failed to create data directory {}", data_dir.display()))?;

        Ok(Self {
            repositories: NamedFileRepositories::new(data_dir),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub(crate) async fn insert_session(&self, repository: RecordsFileRepository) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let persisted_snapshot = repository.persisted_dump().await?;
        self.sessions.write().await.insert(
            session_id.clone(),
            Arc::new(Mutex::new(SessionState {
                repository,
                persisted_snapshot,
            })),
        );
        Ok(session_id)
    }

    pub(crate) async fn get_session(
        &self,
        session_id: &str,
    ) -> std::result::Result<Arc<Mutex<SessionState>>, ApiError> {
        self.sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| ApiError::not_found("Session does not exist"))
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(api::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn rustls_config(
    paths: &ServerConfigPaths,
) -> Result<axum_server::tls_rustls::RustlsConfig> {
    install_crypto_provider();
    let certs = load_certs(&paths.server_cert_pem)?;
    let key = load_private_key(&paths.server_key_pem)?;

    let mut client_roots = RootCertStore::empty();
    for cert in load_certs(&paths.client_ca_cert_pem)? {
        client_roots
            .add(cert)
            .map_err(|err| anyhow!("Failed to add client CA certificate: {err}"))?;
    }

    let verifier = WebPkiClientVerifier::builder(Arc::new(client_roots))
        .build()
        .context("Failed to build client certificate verifier")?;

    let server_config = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .context("Failed to build rustls server config")?;

    Ok(axum_server::tls_rustls::RustlsConfig::from_config(
        Arc::new(server_config),
    ))
}

pub async fn serve(config: ServerConfigPaths) -> Result<()> {
    install_crypto_provider();
    let state = AppState::new(config.data_dir.clone()).await?;
    let tls_config = rustls_config(&config).await?;
    axum_server::bind_rustls(config.bind_addr, tls_config)
        .serve(app(state).into_make_service())
        .await
        .context("Server exited with error")
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!("internal API error: {error}");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal server error".into(),
        }
    }

    fn from_create_error(error: CreateRepositoryError) -> Self {
        match error {
            CreateRepositoryError::RepositoryAlreadyExists => Self {
                status: StatusCode::CONFLICT,
                message: "Repository already exists".into(),
            },
            CreateRepositoryError::InvalidRepositoryName(name) => {
                Self::bad_request(format!("Invalid repository name: {name}"))
            }
            CreateRepositoryError::UnexpectedError(err) => Self::internal(err),
        }
    }

    fn from_open_error(error: RepositoryOpenError) -> Self {
        match error {
            RepositoryOpenError::WrongPassword => Self {
                status: StatusCode::UNAUTHORIZED,
                message: "Wrong password".into(),
            },
            RepositoryOpenError::DoesntExist => Self::not_found("Repository does not exist"),
            RepositoryOpenError::InvalidRepositoryName(name) => {
                Self::bad_request(format!("Invalid repository name: {name}"))
            }
            RepositoryOpenError::OpenError(err) => Self::internal(err),
        }
    }

    fn from_add_error(error: sec_store::repository::AddRecordError) -> Self {
        match error {
            sec_store::repository::AddRecordError::RecordDoesntExist => Self {
                status: StatusCode::CONFLICT,
                message: "Record already exists".into(),
            },
            sec_store::repository::AddRecordError::UnxpectedError(err) => Self::internal(err),
        }
    }

    fn from_update_error(error: UpdateRecordError) -> Self {
        match error {
            UpdateRecordError::RecordDoesntExist => Self::not_found("Record does not exist"),
            UpdateRecordError::UnxpectedError(err) => Self::internal(err),
        }
    }
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let data = std::fs::read(path)
        .with_context(|| format!("Failed to read certificate file {}", path.display()))?;
    let mut reader = BufReader::new(Cursor::new(data));
    rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to parse PEM certificates")
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let data = std::fs::read(path)
        .with_context(|| format!("Failed to read key file {}", path.display()))?;
    let mut reader = BufReader::new(Cursor::new(data));
    rustls_pemfile::private_key(&mut reader)
        .context("Failed to parse private key PEM")?
        .ok_or_else(|| anyhow!("No private key found in {}", path.display()))
}

fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::net::TcpListener;
    use std::time::Duration;

    use rcgen::{
        BasicConstraints, CertificateParams, CertifiedIssuer, DistinguishedName, DnType, IsCa,
        KeyPair, SanType,
    };
    use reqwest::{Certificate, Identity};
    use sec_store::repository::remote::{
        CreateRepositoryRequest, OpenRepositoryRequest, OpenRepositoryResponse,
    };
    use tempfile::TempDir;

    use super::*;

    pub(crate) struct TestServer {
        pub(crate) base_url: String,
        client_identity_path: PathBuf,
        ca_cert_path: PathBuf,
        _tmp: TempDir,
    }

    pub(crate) async fn create_repo(
        client: &reqwest::Client,
        server: &TestServer,
        name: &str,
        password: &str,
    ) {
        let response = client
            .post(format!("{}/repositories/{}", server.base_url, name))
            .json(&CreateRepositoryRequest {
                password: password.to_string(),
            })
            .send()
            .await
            .expect("create response");
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    pub(crate) async fn open_session(
        client: &reqwest::Client,
        server: &TestServer,
        name: &str,
        password: &str,
    ) -> OpenRepositoryResponse {
        client
            .post(format!(
                "{}/repositories/{}/sessions",
                server.base_url, name
            ))
            .json(&OpenRepositoryRequest {
                password: password.to_string(),
            })
            .send()
            .await
            .expect("open response")
            .json::<OpenRepositoryResponse>()
            .await
            .expect("open json")
    }

    pub(crate) async fn spawn_test_server() -> Result<TestServer> {
        let tmp = TempDir::new().context("temp dir")?;
        let certs = TestCertificates::generate(tmp.path())?;

        let listener = TcpListener::bind("127.0.0.1:0").context("bind listener")?;
        let addr = listener.local_addr().context("local addr")?;
        drop(listener);

        let config = ServerConfigPaths {
            bind_addr: addr,
            data_dir: tmp.path().join("data"),
            server_cert_pem: certs.server_cert_path.clone(),
            server_key_pem: certs.server_key_path.clone(),
            client_ca_cert_pem: certs.ca_cert_path.clone(),
        };

        tokio::spawn(async move {
            let _ = serve(config).await;
        });

        tokio::time::sleep(Duration::from_millis(300)).await;

        Ok(TestServer {
            base_url: format!("https://{}", addr),
            client_identity_path: certs.client_identity_path,
            ca_cert_path: certs.ca_cert_path,
            _tmp: tmp,
        })
    }

    pub(crate) async fn build_client(
        server: &TestServer,
        trusted: bool,
    ) -> Result<reqwest::Client> {
        install_crypto_provider();
        let identity_path = if trusted {
            server.client_identity_path.clone()
        } else {
            let untrusted = server
                .client_identity_path
                .parent()
                .unwrap()
                .join("untrusted-client.pem");
            TestCertificates::write_untrusted_client(&untrusted)?;
            untrusted
        };

        let identity = Identity::from_pem(
            &tokio::fs::read(identity_path)
                .await
                .context("read client identity")?,
        )
        .context("parse client identity")?;
        let ca_cert = Certificate::from_pem(
            &tokio::fs::read(&server.ca_cert_path)
                .await
                .context("read ca cert")?,
        )
        .context("parse ca cert")?;

        reqwest::Client::builder()
            .identity(identity)
            .add_root_certificate(ca_cert)
            .use_rustls_tls()
            .https_only(true)
            .build()
            .context("build reqwest client")
    }

    struct TestCertificates {
        ca_cert_path: PathBuf,
        server_cert_path: PathBuf,
        server_key_path: PathBuf,
        client_identity_path: PathBuf,
    }

    impl TestCertificates {
        fn generate(base_dir: &Path) -> Result<Self> {
            let mut ca_params = CertificateParams::new(Vec::<String>::new())?;
            ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            ca_params.distinguished_name = DistinguishedName::new();
            ca_params
                .distinguished_name
                .push(DnType::CommonName, "PasswordsKeeper Test CA");
            let ca_key = KeyPair::generate()?;
            let ca = CertifiedIssuer::self_signed(ca_params, ca_key)?;

            let server = generate_signed_cert(
                &ca,
                "localhost",
                vec![
                    SanType::DnsName("localhost".try_into()?),
                    SanType::IpAddress("127.0.0.1".parse()?),
                ],
            )?;
            let client = generate_signed_cert(&ca, "allowed-client", Vec::new())?;

            let ca_cert_path = base_dir.join("ca.pem");
            let server_cert_path = base_dir.join("server.pem");
            let server_key_path = base_dir.join("server-key.pem");
            let client_identity_path = base_dir.join("client-identity.pem");

            std::fs::write(&ca_cert_path, ca.pem()).context("write ca cert")?;
            std::fs::write(&server_cert_path, server.cert.pem()).context("write server cert")?;
            std::fs::write(&server_key_path, server.key_pair.serialize_pem())
                .context("write server key")?;
            std::fs::write(
                &client_identity_path,
                format!("{}{}", client.cert.pem(), client.key_pair.serialize_pem()),
            )
            .context("write client identity")?;

            Ok(Self {
                ca_cert_path,
                server_cert_path,
                server_key_path,
                client_identity_path,
            })
        }

        fn write_untrusted_client(path: &Path) -> Result<()> {
            let cert = generate_self_signed_cert("untrusted-client")?;
            std::fs::write(
                path,
                format!("{}{}", cert.cert.pem(), cert.key_pair.serialize_pem()),
            )
            .with_context(|| format!("write {}", path.display()))
        }
    }

    struct GeneratedCert {
        cert: rcgen::Certificate,
        key_pair: KeyPair,
    }

    fn generate_signed_cert(
        issuer: &CertifiedIssuer<'_, KeyPair>,
        common_name: &str,
        subject_alt_names: Vec<SanType>,
    ) -> Result<GeneratedCert> {
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params.subject_alt_names = subject_alt_names;
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, common_name);
        let key_pair = KeyPair::generate()?;
        let cert = params.signed_by(&key_pair, issuer)?;
        Ok(GeneratedCert { cert, key_pair })
    }

    fn generate_self_signed_cert(common_name: &str) -> Result<GeneratedCert> {
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, common_name);
        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;
        Ok(GeneratedCert { cert, key_pair })
    }
}
