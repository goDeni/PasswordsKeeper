use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::ValueEnum;
use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
use sec_store::repository::remote::{
    RemoteClientConfig, RemoteRecordsRepository, RemoteRepositoriesClient,
};
use sec_store::repository::{
    AddResult, CreateRepositoryError, OpenRepository, RecordsRepository, RepositoriesSource,
    RepositoryOpenError, UpdateResult,
};
use serde::Deserialize;

use crate::runtime::block_on;

static REPOSITORY_SOURCE_OVERRIDE: OnceLock<Mutex<Option<RepositorySource>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ConnectionMode {
    File,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRepositoryConfig {
    pub base_url: String,
    pub client_identity_pem_path: PathBuf,
    pub ca_cert_pem_path: PathBuf,
    pub repository_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositorySource {
    File { repo_path: PathBuf },
    Remote(RemoteRepositoryConfig),
}

#[derive(Debug, Clone)]
pub enum TuiRepository {
    File(RecordsFileRepository),
    Remote(RemoteRecordsRepository),
}

#[derive(Debug, Deserialize)]
struct RawRemoteRepositoryTomlConfig {
    base_url: String,
    client_identity_pem_path: PathBuf,
    ca_cert_pem_path: PathBuf,
    repository_name: String,
}

fn configured_repository_source() -> Option<RepositorySource> {
    REPOSITORY_SOURCE_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn resolve_from_config_dir(base_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn repository_source() -> RepositorySource {
    configured_repository_source().unwrap_or_else(|| RepositorySource::File {
        repo_path: default_repo_path(),
    })
}

fn build_remote_client(config: &RemoteRepositoryConfig) -> Result<RemoteRepositoriesClient> {
    block_on(RemoteRepositoriesClient::from_config(RemoteClientConfig {
        base_url: config.base_url.clone(),
        client_identity_pem_path: config
            .client_identity_pem_path
            .to_string_lossy()
            .into_owned(),
        ca_cert_pem_path: config.ca_cert_pem_path.to_string_lossy().into_owned(),
    }))
}

fn map_create_error(error: CreateRepositoryError) -> anyhow::Error {
    match error {
        CreateRepositoryError::RepositoryAlreadyExists => {
            anyhow::anyhow!("Repository already exists")
        }
        CreateRepositoryError::InvalidRepositoryName(name) => {
            anyhow::anyhow!("Invalid repository name: {name}")
        }
        CreateRepositoryError::UnexpectedError(err) => err,
    }
}

fn map_open_error(error: RepositoryOpenError) -> anyhow::Error {
    match error {
        RepositoryOpenError::WrongPassword => anyhow::anyhow!("Wrong password"),
        RepositoryOpenError::DoesntExist => {
            anyhow::anyhow!("Repository does not exist. Create one first.")
        }
        RepositoryOpenError::InvalidRepositoryName(name) => {
            anyhow::anyhow!("Invalid repository name: {name}")
        }
        RepositoryOpenError::OpenError(err) => err,
    }
}

impl TuiRepository {
    pub async fn close_connection(&self) -> Result<()> {
        match self.clone() {
            Self::File(_) => Ok(()),
            Self::Remote(repo) => repo.close().await,
        }
    }
}

#[async_trait]
impl RecordsRepository for TuiRepository {
    async fn cancel(&mut self) -> Result<()> {
        match self {
            Self::File(repo) => repo.cancel().await,
            Self::Remote(repo) => repo.cancel().await,
        }
    }

    async fn save(&mut self) -> Result<()> {
        match self {
            Self::File(repo) => repo.save().await,
            Self::Remote(repo) => repo.save().await,
        }
    }

    async fn get_records(&self) -> Result<Vec<sec_store::record::Record>> {
        match self {
            Self::File(repo) => repo.get_records().await,
            Self::Remote(repo) => repo.get_records().await,
        }
    }

    async fn get(
        &self,
        record_id: &sec_store::record::RecordId,
    ) -> Result<Option<sec_store::record::Record>> {
        match self {
            Self::File(repo) => repo.get(record_id).await,
            Self::Remote(repo) => repo.get(record_id).await,
        }
    }

    async fn update(&mut self, record: sec_store::record::Record) -> UpdateResult<()> {
        match self {
            Self::File(repo) => repo.update(record).await,
            Self::Remote(repo) => repo.update(record).await,
        }
    }

    async fn delete(&mut self, record_id: &sec_store::record::RecordId) -> UpdateResult<()> {
        match self {
            Self::File(repo) => repo.delete(record_id).await,
            Self::Remote(repo) => repo.delete(record_id).await,
        }
    }

    async fn add_record(&mut self, record: sec_store::record::Record) -> AddResult<()> {
        match self {
            Self::File(repo) => repo.add_record(record).await,
            Self::Remote(repo) => repo.add_record(record).await,
        }
    }

    async fn dump(&self) -> Result<Vec<u8>> {
        match self {
            Self::File(repo) => repo.dump().await,
            Self::Remote(repo) => repo.dump().await,
        }
    }
}

pub fn default_repo_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join("passwords_keeper_tui_data")
        .join("repo")
}

pub fn resolve_repo_path(cli_repo_path: PathBuf) -> PathBuf {
    cli_repo_path
}

pub fn resolve_data_dir(repo_path: &Path) -> PathBuf {
    repo_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load_remote_repository_config(path: impl AsRef<Path>) -> Result<RemoteRepositoryConfig> {
    let path = path.as_ref();
    let config_contents = fs::read_to_string(path)
        .with_context(|| format!("read remote repository config {}", path.display()))?;
    let raw: RawRemoteRepositoryTomlConfig = toml::from_str(&config_contents)
        .with_context(|| format!("parse remote repository config {}", path.display()))?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

    Ok(RemoteRepositoryConfig {
        base_url: raw.base_url,
        client_identity_pem_path: resolve_from_config_dir(base_dir, raw.client_identity_pem_path),
        ca_cert_pem_path: resolve_from_config_dir(base_dir, raw.ca_cert_pem_path),
        repository_name: raw.repository_name,
    })
}

pub fn configure_repository_source(source: RepositorySource) {
    *REPOSITORY_SOURCE_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(source);
}

pub fn configure_repo_path(repo_path: PathBuf) {
    configure_repository_source(RepositorySource::File { repo_path });
}

#[cfg(test)]
pub(crate) fn clear_configured_repo_path() {
    *REPOSITORY_SOURCE_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

fn data_dir() -> PathBuf {
    match repository_source() {
        RepositorySource::File { repo_path } => resolve_data_dir(&repo_path),
        RepositorySource::Remote(_) => std::env::current_dir().unwrap_or_default(),
    }
}

fn repo_path() -> PathBuf {
    match repository_source() {
        RepositorySource::File { repo_path } => repo_path,
        RepositorySource::Remote(_) => default_repo_path(),
    }
}

pub fn ensure_data_dir() -> Result<PathBuf> {
    let d = data_dir();
    if !d.exists() {
        fs::create_dir_all(&d).with_context(|| format!("create data dir {:?}", d))?;
    }
    Ok(d)
}

pub fn has_repo() -> bool {
    match repository_source() {
        RepositorySource::File { .. } => repo_path().exists(),
        RepositorySource::Remote(_) => true,
    }
}

pub fn create_repo(password: String) -> Result<TuiRepository> {
    match repository_source() {
        RepositorySource::File { repo_path } => {
            ensure_data_dir()?;
            let mut repo = RecordsFileRepository::new(repo_path, password);
            block_on(repo.save())?;
            Ok(TuiRepository::File(repo))
        }
        RepositorySource::Remote(config) => {
            let client = build_remote_client(&config)?;
            let repo = block_on(client.create_repository(&config.repository_name, password))
                .map_err(map_create_error)?;
            Ok(TuiRepository::Remote(repo))
        }
    }
}

pub fn open_repo(password: String) -> Result<TuiRepository> {
    match repository_source() {
        RepositorySource::File { repo_path } => {
            let repo = block_on(OpenRecordsFileRepository(repo_path).open(password))
                .map_err(map_open_error)?;
            Ok(TuiRepository::File(repo))
        }
        RepositorySource::Remote(config) => {
            let client = build_remote_client(&config)?;
            let repo = block_on(client.open_repository(&config.repository_name, password))
                .map_err(map_open_error)?;
            Ok(TuiRepository::Remote(repo))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use clap::ValueEnum;
    use tempfile::TempDir;

    use crate::test_helpers::{test_password, ScopedTuiDataDir};

    use super::{
        clear_configured_repo_path, configure_repo_path, configure_repository_source, create_repo,
        default_repo_path, ensure_data_dir, has_repo, load_remote_repository_config, open_repo,
        resolve_data_dir, resolve_repo_path, ConnectionMode, RemoteRepositoryConfig,
        RepositorySource,
    };

    #[test]
    fn test_default_repo_path_uses_default_location() {
        let path = default_repo_path();
        assert!(path.ends_with("passwords_keeper_tui_data/repo"));
    }

    #[test]
    fn test_connection_mode_values_stable() {
        assert_eq!(
            ConnectionMode::File.to_possible_value().unwrap().get_name(),
            "file"
        );
        assert_eq!(
            ConnectionMode::Remote
                .to_possible_value()
                .unwrap()
                .get_name(),
            "remote"
        );
    }

    #[test]
    fn test_resolve_data_dir_uses_parent_directory() {
        let path = resolve_data_dir(Path::new("/tmp/custom/repo-file"));
        assert_eq!(path, PathBuf::from("/tmp/custom"));
    }

    #[test]
    fn test_resolve_repo_path_prefers_cli_repo_file() {
        let path = resolve_repo_path(PathBuf::from("/tmp/custom-repo"));
        assert_eq!(path, PathBuf::from("/tmp/custom-repo"));
    }

    #[test]
    fn test_ensure_data_dir_creates_directory() {
        let scope = ScopedTuiDataDir::new();
        let data_dir = ensure_data_dir().expect("failed to create data dir");
        assert_eq!(data_dir, scope.temp_dir.path());
        assert!(data_dir.exists());
        assert!(data_dir.is_dir());
    }

    #[test]
    fn test_has_repo_false_before_creation() {
        let _scope = ScopedTuiDataDir::new();
        assert!(!has_repo());
    }

    #[test]
    fn test_create_repo_creates_file() {
        let scope = ScopedTuiDataDir::new();
        create_repo(test_password()).expect("repo should be created");

        let repo_file = scope.temp_dir.path().join("repo");
        assert!(repo_file.exists());
    }

    #[test]
    fn test_create_repo_uses_configured_repo_path() {
        let _scope = ScopedTuiDataDir::new();
        let repo_file = std::env::temp_dir().join("passwordskeeper-custom-repo-file");
        if repo_file.exists() {
            std::fs::remove_file(&repo_file).expect("remove stale repo file");
        }

        configure_repo_path(repo_file.clone());
        create_repo(test_password()).expect("repo should be created");
        assert!(repo_file.exists());

        std::fs::remove_file(&repo_file).expect("remove repo file");
        clear_configured_repo_path();
    }

    #[test]
    fn test_has_repo_true_after_creation() {
        let _scope = ScopedTuiDataDir::new();
        create_repo(test_password()).expect("repo should be created");
        assert!(has_repo());
    }

    #[test]
    fn test_open_repo_success() {
        let _scope = ScopedTuiDataDir::new();
        let password = test_password();
        create_repo(password.clone()).expect("repo should be created");
        open_repo(password).expect("repo should open");
    }

    #[test]
    fn test_open_repo_wrong_password_error() {
        let _scope = ScopedTuiDataDir::new();
        create_repo(test_password()).expect("repo should be created");
        let err = open_repo("wrong".to_string()).expect_err("open should fail");
        assert!(err.to_string().contains("Wrong password"));
    }

    #[test]
    fn test_open_repo_missing_repo_error() {
        let _scope = ScopedTuiDataDir::new();
        let err = open_repo(test_password()).expect_err("open should fail");
        assert!(err
            .to_string()
            .contains("Repository does not exist. Create one first."));
    }

    #[test]
    fn test_load_remote_repository_config_resolves_relative_paths() {
        let temp_dir = TempDir::new().expect("temp dir");
        let config_path = temp_dir.path().join("remote.toml");
        fs::write(
            &config_path,
            r#"base_url = "https://server.example"
client_identity_pem_path = "certs/client.pem"
ca_cert_pem_path = "certs/ca.pem"
repository_name = "demo"
"#,
        )
        .expect("write config");

        let config = load_remote_repository_config(&config_path).expect("load config");
        assert_eq!(
            config,
            RemoteRepositoryConfig {
                base_url: "https://server.example".to_string(),
                client_identity_pem_path: temp_dir.path().join("certs/client.pem"),
                ca_cert_pem_path: temp_dir.path().join("certs/ca.pem"),
                repository_name: "demo".to_string(),
            }
        );
    }

    #[test]
    fn test_has_repo_true_for_remote_mode() {
        configure_repository_source(RepositorySource::Remote(RemoteRepositoryConfig {
            base_url: "https://server.example".to_string(),
            client_identity_pem_path: PathBuf::from("/tmp/client.pem"),
            ca_cert_pem_path: PathBuf::from("/tmp/ca.pem"),
            repository_name: "demo".to_string(),
        }));

        assert!(has_repo());

        clear_configured_repo_path();
    }
}
