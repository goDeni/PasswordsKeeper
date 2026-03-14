use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
use sec_store::repository::{OpenRepository, RecordsRepository, RepositoryOpenError};

use crate::runtime::block_on;

static REPO_PATH_OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn configured_repo_path() -> Option<PathBuf> {
    REPO_PATH_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
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

pub fn configure_repo_path(repo_path: PathBuf) {
    *REPO_PATH_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(repo_path);
}

#[cfg(test)]
pub(crate) fn clear_configured_repo_path() {
    *REPO_PATH_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

fn data_dir() -> PathBuf {
    configured_repo_path()
        .map(|path| resolve_data_dir(&path))
        .unwrap_or_else(|| resolve_data_dir(&default_repo_path()))
}

fn repo_path() -> PathBuf {
    configured_repo_path().unwrap_or_else(default_repo_path)
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
    block_on(repo.save())?;
    Ok(repo)
}

pub fn open_repo(password: String) -> Result<RecordsFileRepository> {
    match block_on(OpenRecordsFileRepository(repo_path()).open(password)) {
        Ok(r) => Ok(r),
        Err(RepositoryOpenError::WrongPassword) => {
            anyhow::bail!("Wrong password")
        }
        Err(RepositoryOpenError::DoesntExist) => {
            anyhow::bail!("Repository does not exist. Create one first.")
        }
        Err(RepositoryOpenError::InvalidRepositoryName(name)) => {
            anyhow::bail!("Invalid repository name: {}", name)
        }
        Err(RepositoryOpenError::OpenError(e)) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::{test_password, ScopedTuiDataDir};

    use super::{
        clear_configured_repo_path, configure_repo_path, create_repo, default_repo_path,
        ensure_data_dir, has_repo, open_repo, resolve_data_dir, resolve_repo_path,
    };

    #[test]
    fn test_default_repo_path_uses_default_location() {
        let path = default_repo_path();
        assert!(path.ends_with("passwords_keeper_tui_data/repo"));
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

    use std::path::{Path, PathBuf};
}
