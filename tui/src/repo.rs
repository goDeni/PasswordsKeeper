use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
use sec_store::repository::{OpenRepository, RecordsRepository, RepositoryOpenError};

static DATA_DIR_OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

/// Data directory for TUI (e.g. ./passwords_keeper_tui_data or PASSWORDS_KEEPER_TUI_DATA).
fn configured_data_dir() -> Option<PathBuf> {
    DATA_DIR_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

pub fn resolve_data_dir(cli_data_dir: Option<PathBuf>) -> PathBuf {
    cli_data_dir
        .or_else(|| std::env::var_os("PASSWORDS_KEEPER_TUI_DATA").map(PathBuf::from))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("passwords_keeper_tui_data")
        })
}

pub fn configure_data_dir(data_dir: PathBuf) {
    *DATA_DIR_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(data_dir);
}

fn data_dir() -> PathBuf {
    configured_data_dir().unwrap_or_else(|| resolve_data_dir(None))
}

fn repo_path() -> PathBuf {
    data_dir().join("repo")
}

pub fn ensure_data_dir() -> Result<PathBuf> {
    let d = data_dir();
    if !d.exists() {
        fs::create_dir_all(&d).with_context(|| format!("create data dir {:?}", d))?;
    }
    Ok(d)
}

pub fn has_repo() -> bool {
    repo_path().exists()
}

pub fn create_repo(password: String) -> Result<RecordsFileRepository> {
    ensure_data_dir()?;
    let path = repo_path();
    let mut repo = RecordsFileRepository::new(path, password);
    repo.save()?;
    Ok(repo)
}

pub fn open_repo(password: String) -> Result<RecordsFileRepository> {
    match OpenRecordsFileRepository(repo_path()).open(password) {
        Ok(r) => Ok(r),
        Err(RepositoryOpenError::WrongPassword) => {
            anyhow::bail!("Wrong password")
        }
        Err(RepositoryOpenError::DoesntExist) => {
            anyhow::bail!("Repository does not exist. Create one first.")
        }
        Err(RepositoryOpenError::OpenError(e)) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::ScopedTuiDataDir;

    use super::{create_repo, ensure_data_dir, has_repo, open_repo, resolve_data_dir};

    #[test]
    fn test_resolve_data_dir_prefers_cli_over_env() {
        let _scope = ScopedTuiDataDir::new();
        let path = resolve_data_dir(Some("/tmp/cli-path".into()));
        assert_eq!(path, PathBuf::from("/tmp/cli-path"));
    }

    #[test]
    fn test_resolve_data_dir_uses_env_when_cli_missing() {
        let scope = ScopedTuiDataDir::new();
        let path = resolve_data_dir(None);
        assert_eq!(path, scope.temp_dir.path());
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
        create_repo("password".to_string()).expect("repo should be created");

        let repo_file = scope.temp_dir.path().join("repo");
        assert!(repo_file.exists());
    }

    #[test]
    fn test_has_repo_true_after_creation() {
        let _scope = ScopedTuiDataDir::new();
        create_repo("password".to_string()).expect("repo should be created");
        assert!(has_repo());
    }

    #[test]
    fn test_open_repo_success() {
        let _scope = ScopedTuiDataDir::new();
        create_repo("password".to_string()).expect("repo should be created");
        open_repo("password".to_string()).expect("repo should open");
    }

    #[test]
    fn test_open_repo_wrong_password_error() {
        let _scope = ScopedTuiDataDir::new();
        create_repo("password".to_string()).expect("repo should be created");
        let err = open_repo("wrong".to_string()).expect_err("open should fail");
        assert!(err.to_string().contains("Wrong password"));
    }

    #[test]
    fn test_open_repo_missing_repo_error() {
        let _scope = ScopedTuiDataDir::new();
        let err = open_repo("password".to_string()).expect_err("open should fail");
        assert!(err
            .to_string()
            .contains("Repository does not exist. Create one first."));
    }

    use std::path::PathBuf;
}
