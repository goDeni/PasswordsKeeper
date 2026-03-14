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

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
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
mod tests {
    use std::net::TcpListener;
    use std::time::Duration;

    use rcgen::{
        BasicConstraints, CertificateParams, CertifiedIssuer, DistinguishedName, DnType, IsCa,
        KeyPair, SanType,
    };
    use reqwest::{Certificate, Identity};
    use sec_store::record::Record;
    use sec_store::repository::remote::{
        AddRecordRequest, CreateRepositoryRequest, OpenRepositoryRequest, OpenRepositoryResponse,
    };
    use sec_store::repository::{OpenRepository, RecordsRepository};
    use tempfile::TempDir;

    use super::*;

    struct TestServer {
        base_url: String,
        client_identity_path: PathBuf,
        ca_cert_path: PathBuf,
        _tmp: TempDir,
    }

    #[tokio::test]
    async fn mtls_server_rejects_unknown_client_and_persists_records() {
        let server = spawn_test_server().await.expect("server");
        let allowed_client = build_client(&server, true).await.expect("allowed client");

        let create_response = allowed_client
            .post(format!("{}/repositories/demo", server.base_url))
            .json(&CreateRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("create response");
        assert_eq!(create_response.status(), StatusCode::CREATED);

        let session = allowed_client
            .post(format!("{}/repositories/demo/sessions", server.base_url))
            .json(&OpenRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("open response")
            .json::<OpenRepositoryResponse>()
            .await
            .expect("open json");

        let record = Record::new(vec![("name".to_string(), "mail".to_string())]);
        let add_response = allowed_client
            .post(format!(
                "{}/sessions/{}/records",
                server.base_url, session.session_id
            ))
            .json(&AddRecordRequest {
                record: record.clone(),
            })
            .send()
            .await
            .expect("add response");
        assert_eq!(add_response.status(), StatusCode::CREATED);

        let save_response = allowed_client
            .post(format!(
                "{}/sessions/{}/save",
                server.base_url, session.session_id
            ))
            .send()
            .await
            .expect("save response");
        assert_eq!(save_response.status(), StatusCode::OK);

        let second_session = allowed_client
            .post(format!("{}/repositories/demo/sessions", server.base_url))
            .json(&OpenRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("second open response")
            .json::<OpenRepositoryResponse>()
            .await
            .expect("second open json");

        let records = allowed_client
            .get(format!(
                "{}/sessions/{}/records",
                server.base_url, second_session.session_id
            ))
            .send()
            .await
            .expect("records response")
            .json::<Vec<Record>>()
            .await
            .expect("records json");
        assert_eq!(records, vec![record]);

        let denied_client = build_client(&server, false).await.expect("denied client");
        let denied_result = denied_client
            .get(format!(
                "{}/sessions/{}/records",
                server.base_url, second_session.session_id
            ))
            .send()
            .await;
        assert!(denied_result.is_err());
    }

    #[tokio::test]
    async fn export_uses_unsaved_session_state() {
        let server = spawn_test_server().await.expect("server");
        let client = build_client(&server, true).await.expect("client");

        create_repo(&client, &server, "demo", "secret").await;
        let session = open_session(&client, &server, "demo", "secret").await;

        let record = Record::new(vec![("name".to_string(), "draft".to_string())]);
        let add_response = client
            .post(format!(
                "{}/sessions/{}/records",
                server.base_url, session.session_id
            ))
            .json(&AddRecordRequest {
                record: record.clone(),
            })
            .send()
            .await
            .expect("add response");
        assert_eq!(add_response.status(), StatusCode::CREATED);

        let export_response = client
            .get(format!(
                "{}/sessions/{}/export",
                server.base_url, session.session_id
            ))
            .send()
            .await
            .expect("export response");
        assert_eq!(export_response.status(), StatusCode::OK);
        let dump = export_response.bytes().await.expect("dump bytes");

        let temp_dir = TempDir::new().expect("temp dir");
        let dump_path = temp_dir.path().join("repo.json");
        std::fs::write(&dump_path, dump).expect("write dump");
        let dumped_repo = sec_store::repository::file::OpenRecordsFileRepository(dump_path)
            .open("secret".to_string())
            .await
            .expect("open dumped repo");
        let records = dumped_repo.get_records().await.expect("records");
        assert_eq!(records, vec![record]);
    }

    #[tokio::test]
    async fn concurrent_save_returns_conflict_instead_of_overwriting() {
        let server = spawn_test_server().await.expect("server");
        let client = build_client(&server, true).await.expect("client");

        create_repo(&client, &server, "demo", "secret").await;
        let first = open_session(&client, &server, "demo", "secret").await;
        let second = open_session(&client, &server, "demo", "secret").await;

        let first_record = Record::new(vec![("name".to_string(), "first".to_string())]);
        let second_record = Record::new(vec![("name".to_string(), "second".to_string())]);

        let first_add = client
            .post(format!(
                "{}/sessions/{}/records",
                server.base_url, first.session_id
            ))
            .json(&AddRecordRequest {
                record: first_record.clone(),
            })
            .send()
            .await
            .expect("first add");
        assert_eq!(first_add.status(), StatusCode::CREATED);

        let second_add = client
            .post(format!(
                "{}/sessions/{}/records",
                server.base_url, second.session_id
            ))
            .json(&AddRecordRequest {
                record: second_record,
            })
            .send()
            .await
            .expect("second add");
        assert_eq!(second_add.status(), StatusCode::CREATED);

        let first_save = client
            .post(format!(
                "{}/sessions/{}/save",
                server.base_url, first.session_id
            ))
            .send()
            .await
            .expect("first save");
        assert_eq!(first_save.status(), StatusCode::OK);

        let second_save = client
            .post(format!(
                "{}/sessions/{}/save",
                server.base_url, second.session_id
            ))
            .send()
            .await
            .expect("second save");
        assert_eq!(second_save.status(), StatusCode::CONFLICT);

        let verify = open_session(&client, &server, "demo", "secret").await;
        let records = client
            .get(format!(
                "{}/sessions/{}/records",
                server.base_url, verify.session_id
            ))
            .send()
            .await
            .expect("records response")
            .json::<Vec<Record>>()
            .await
            .expect("records json");
        assert_eq!(records, vec![first_record]);
    }

    #[tokio::test]
    async fn invalid_repository_names_return_bad_request() {
        let server = spawn_test_server().await.expect("server");
        let client = build_client(&server, true).await.expect("client");

        let create_response = client
            .post(format!("{}/repositories/invalid%3Aname", server.base_url))
            .json(&CreateRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("create response");
        assert_eq!(create_response.status(), StatusCode::BAD_REQUEST);

        let open_response = client
            .post(format!(
                "{}/repositories/invalid%3Aname/sessions",
                server.base_url
            ))
            .json(&OpenRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("open response");
        assert_eq!(open_response.status(), StatusCode::BAD_REQUEST);
    }

    async fn create_repo(
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

    async fn open_session(
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

    async fn spawn_test_server() -> Result<TestServer> {
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

    async fn build_client(server: &TestServer, trusted: bool) -> Result<reqwest::Client> {
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
