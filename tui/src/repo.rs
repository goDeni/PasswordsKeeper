use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::ValueEnum;
use sec_store::record::{Record, RecordId};
use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
use sec_store::repository::remote::{
    RemoteClientConfig, RemoteRecordsRepository, RemoteRepositoriesClient,
};
use sec_store::repository::{
    CreateRepositoryError, OpenRepository, RecordsRepository, RepositoriesSource,
    RepositoryOpenError,
};
use serde::Deserialize;

use crate::runtime::block_on;

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

pub trait RepositoryFactory<R>: Debug + Clone + Sync + Send + 'static
where
    R: RecordsRepository,
{
    fn has_repo(&self) -> bool;
    fn create_repo(&self, password: String) -> Result<R>;
    fn open_repo(&self, password: String) -> Result<R>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRepositoryFactory {
    repo_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RemoteRepositoryFactory {
    config: RemoteRepositoryConfig,
    client: RemoteRepositoriesClient,
}

#[derive(Debug, Deserialize)]
struct RawRemoteRepositoryTomlConfig {
    base_url: String,
    client_identity_pem_path: PathBuf,
    ca_cert_pem_path: PathBuf,
    repository_name: String,
}

impl FileRepositoryFactory {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}

impl RemoteRepositoryFactory {
    pub fn new(config: RemoteRepositoryConfig) -> Result<Self> {
        let client = block_on(RemoteRepositoriesClient::from_config(RemoteClientConfig {
            base_url: config.base_url.clone(),
            client_identity_pem_path: config
                .client_identity_pem_path
                .to_string_lossy()
                .into_owned(),
            ca_cert_pem_path: config.ca_cert_pem_path.to_string_lossy().into_owned(),
        }))?;

        Ok(Self { config, client })
    }
}

fn resolve_from_config_dir(base_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn ensure_data_dir_for_path(repo_path: &Path) -> Result<PathBuf> {
    let data_dir = resolve_data_dir(repo_path);
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).with_context(|| format!("create data dir {:?}", data_dir))?;
    }
    Ok(data_dir)
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

impl RepositoryFactory<RecordsFileRepository> for FileRepositoryFactory {
    fn has_repo(&self) -> bool {
        self.repo_path.exists()
    }

    fn create_repo(&self, password: String) -> Result<RecordsFileRepository> {
        ensure_data_dir_for_path(&self.repo_path)?;
        let mut repo = RecordsFileRepository::new(self.repo_path.clone(), password);
        block_on(repo.save())?;
        Ok(repo)
    }

    fn open_repo(&self, password: String) -> Result<RecordsFileRepository> {
        let repo = block_on(OpenRecordsFileRepository(self.repo_path.clone()).open(password))
            .map_err(map_open_error)?;
        Ok(repo)
    }
}

impl RepositoryFactory<RemoteRecordsRepository> for RemoteRepositoryFactory {
    fn has_repo(&self) -> bool {
        true
    }

    fn create_repo(&self, password: String) -> Result<RemoteRecordsRepository> {
        block_on(
            self.client
                .create_repository(&self.config.repository_name, password),
        )
        .map_err(map_create_error)
    }

    fn open_repo(&self, password: String) -> Result<RemoteRecordsRepository> {
        block_on(
            self.client
                .open_repository(&self.config.repository_name, password),
        )
        .map_err(map_open_error)
    }
}

pub fn close_connection<R>(repo: &R) -> Result<()>
where
    R: RecordsRepository,
{
    block_on(repo.close())
}

pub fn get_records<R>(repo: &R) -> Result<Vec<Record>>
where
    R: RecordsRepository,
{
    block_on(repo.get_records())
}

pub fn get_record<R>(repo: &R, record_id: &RecordId) -> Result<Option<Record>>
where
    R: RecordsRepository,
{
    block_on(repo.get(record_id))
}

pub fn add_record<R>(repo: &mut R, record: Record) -> Result<(), anyhow::Error>
where
    R: RecordsRepository,
{
    block_on(repo.add_record(record)).map_err(Into::into)
}

pub fn update_record<R>(repo: &mut R, record: Record) -> Result<(), anyhow::Error>
where
    R: RecordsRepository,
{
    block_on(repo.update(record)).map_err(Into::into)
}

pub fn delete_record<R>(repo: &mut R, record_id: &RecordId) -> Result<(), anyhow::Error>
where
    R: RecordsRepository,
{
    block_on(repo.delete(record_id)).map_err(Into::into)
}

pub fn save<R>(repo: &mut R) -> Result<()>
where
    R: RecordsRepository,
{
    block_on(repo.save())
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use clap::ValueEnum;
    use tempfile::TempDir;

    use crate::test_helpers::test_password;

    use super::{
        default_repo_path, ensure_data_dir_for_path, load_remote_repository_config,
        resolve_data_dir, resolve_repo_path, ConnectionMode, FileRepositoryFactory,
        RemoteRepositoryConfig, RepositoryFactory,
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
        let temp_dir = TempDir::new().expect("temp dir");
        let repo_path = temp_dir.path().join("repo");
        let data_dir = ensure_data_dir_for_path(&repo_path).expect("failed to create data dir");
        assert_eq!(data_dir, temp_dir.path());
        assert!(data_dir.exists());
        assert!(data_dir.is_dir());
    }

    #[test]
    fn test_has_repo_false_before_creation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let factory = FileRepositoryFactory::new(temp_dir.path().join("repo"));
        assert!(!factory.has_repo());
    }

    #[test]
    fn test_create_repo_creates_file() {
        let temp_dir = TempDir::new().expect("temp dir");
        let repo_path = temp_dir.path().join("repo");
        let factory = FileRepositoryFactory::new(repo_path.clone());
        factory
            .create_repo(test_password())
            .expect("repo should be created");

        assert!(repo_path.exists());
    }

    #[test]
    fn test_create_repo_uses_configured_repo_path() {
        let temp_dir = TempDir::new().expect("temp dir");
        let repo_file = temp_dir.path().join("passwordskeeper-custom-repo-file");
        let factory = FileRepositoryFactory::new(repo_file.clone());

        factory
            .create_repo(test_password())
            .expect("repo should be created");
        assert!(repo_file.exists());
    }

    #[test]
    fn test_has_repo_true_after_creation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let factory = FileRepositoryFactory::new(temp_dir.path().join("repo"));
        factory
            .create_repo(test_password())
            .expect("repo should be created");
        assert!(factory.has_repo());
    }

    #[test]
    fn test_open_repo_success() {
        let temp_dir = TempDir::new().expect("temp dir");
        let factory = FileRepositoryFactory::new(temp_dir.path().join("repo"));
        let password = test_password();
        factory
            .create_repo(password.clone())
            .expect("repo should be created");
        factory.open_repo(password).expect("repo should open");
    }

    #[test]
    fn test_open_repo_wrong_password_error() {
        let temp_dir = TempDir::new().expect("temp dir");
        let factory = FileRepositoryFactory::new(temp_dir.path().join("repo"));
        factory
            .create_repo(test_password())
            .expect("repo should be created");
        let err = factory
            .open_repo("wrong".to_string())
            .expect_err("open should fail");
        assert!(err.to_string().contains("Wrong password"));
    }

    #[test]
    fn test_open_repo_missing_repo_error() {
        let temp_dir = TempDir::new().expect("temp dir");
        let factory = FileRepositoryFactory::new(temp_dir.path().join("repo"));
        let err = factory
            .open_repo(test_password())
            .expect_err("open should fail");
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
            r#"
base_url = "https://127.0.0.1:8443"
client_identity_pem_path = "client.pem"
ca_cert_pem_path = "ca.pem"
repository_name = "demo"
"#,
        )
        .expect("write config");

        let config = load_remote_repository_config(&config_path).expect("config should parse");

        assert_eq!(
            config,
            RemoteRepositoryConfig {
                base_url: "https://127.0.0.1:8443".to_string(),
                client_identity_pem_path: temp_dir.path().join("client.pem"),
                ca_cert_pem_path: temp_dir.path().join("ca.pem"),
                repository_name: "demo".to_string(),
            }
        );
    }
}
