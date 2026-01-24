use std::fs;
use std::path::{Path, PathBuf};

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

pub fn restore_repo(backup_path: &Path, password: String) -> Result<RecordsFileRepository> {
    if !backup_path.exists() {
        anyhow::bail!("Backup file does not exist");
    }
    let path = repo_path();
    ensure_data_dir()?;
    let contents =
        fs::read(backup_path).with_context(|| format!("read backup {:?}", backup_path))?;
    fs::write(&path, contents).with_context(|| format!("write repo {:?}", path))?;
    match OpenRecordsFileRepository(path).open(password) {
        Ok(r) => Ok(r),
        Err(RepositoryOpenError::WrongPassword) => anyhow::bail!("Wrong password"),
        Err(RepositoryOpenError::OpenError(e)) => Err(e),
        Err(RepositoryOpenError::DoesntExist) => unreachable!(),
    }
}

pub fn backup_path() -> PathBuf {
    data_dir().join("backup.json")
}
