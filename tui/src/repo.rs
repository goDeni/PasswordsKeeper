use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
use sec_store::repository::{OpenRepository, RecordsRepository, RepositoryOpenError};

/// Data directory for TUI (e.g. ./passwords_keeper_tui_data or PASSWORDS_KEEPER_TUI_DATA).
fn data_dir() -> PathBuf {
    std::env::var_os("PASSWORDS_KEEPER_TUI_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("passwords_keeper_tui_data")
        })
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

    use super::{create_repo, ensure_data_dir, has_repo, open_repo};

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
}
