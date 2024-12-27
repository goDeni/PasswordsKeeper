pub mod file;

use anyhow::Result;
use sec_store::repository::OpenResult;
use std::fmt::Display;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryAlreadyExist;
pub type InitRepoResult<T> = Result<T, RepositoryAlreadyExist>;

#[derive(Debug, Clone, PartialEq)]
pub enum GetReposityError {
    WrongPassword,
    UnexpectedError,
}

#[derive(Debug, Error)]
pub enum RepositoryLoadError {
    WrongPassword,
    OpenError(anyhow::Error),
    UnexpectedError(anyhow::Error),
}
pub type LoadResult<T> = Result<T, RepositoryLoadError>;

impl Display for RepositoryLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryLoadError::WrongPassword => write!(f, "WrongPassword"),
            RepositoryLoadError::OpenError(err) => {
                write!(f, "OpenError({}, {})", err, err.root_cause())
            }
            RepositoryLoadError::UnexpectedError(err) => {
                write!(f, "UnexpectedError({}, {})", err, err.root_cause())
            }
        }
    }
}

pub type UserId = String;
pub trait RepositoriesFactory<T>: Clone + Sync + Send + 'static {
    fn user_has_repository(&self, user_id: &UserId) -> bool;
    fn get_user_repository(&self, user_id: &UserId, passwd: String) -> OpenResult<T>;
    fn load_user_repository<P: AsRef<Path>>(
        &self,
        user_id: &UserId,
        passwd: String,
        file: P,
    ) -> LoadResult<T>;
    fn initialize_user_repository(&self, user_id: &UserId, passwd: String) -> InitRepoResult<T>;
}
