pub mod file;
pub mod remote;

use std::fmt::{Debug, Display};

use crate::record::{Record, RecordId};
use anyhow::{Error, Result};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateRecordError {
    #[error("Record doesn't exist")]
    RecordDoesntExist,
    #[error("Unexpected error: {0}")]
    UnxpectedError(Error),
}
pub type UpdateResult<T> = Result<T, UpdateRecordError>;

#[derive(Debug, Error)]
pub enum AddRecordError {
    #[error("Record doesn't exist")]
    RecordDoesntExist,
    #[error("Unexpected error: {0}")]
    UnxpectedError(Error),
}
pub type AddResult<T> = Result<T, AddRecordError>;

#[derive(Debug, Error)]
pub enum CreateRepositoryError {
    #[error("Repository already exists")]
    RepositoryAlreadyExists,
    #[error("Invalid repository name: {0}")]
    InvalidRepositoryName(String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(Error),
}
pub type CreateRepositoryResult<T> = Result<T, CreateRepositoryError>;

#[derive(Debug, Error)]
pub enum RepositoryOpenError {
    WrongPassword,
    DoesntExist,
    InvalidRepositoryName(String),
    OpenError(anyhow::Error),
}
pub type OpenResult<T> = Result<T, RepositoryOpenError>;

impl Display for RepositoryOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryOpenError::WrongPassword => write!(f, "WrongPassword"),
            RepositoryOpenError::DoesntExist => write!(f, "DoesntExist"),
            RepositoryOpenError::InvalidRepositoryName(name) => {
                write!(f, "InvalidRepositoryName({name})")
            }
            RepositoryOpenError::OpenError(err) => {
                write!(f, "OpenError({}, {})", err, err.root_cause())
            }
        }
    }
}

#[async_trait]
pub trait RecordsRepository: Debug + Clone + Sync + Send + 'static {
    async fn close(&self) -> Result<()> {
        Ok(())
    }
    async fn cancel(&mut self) -> Result<()>;
    async fn save(&mut self) -> Result<()>;
    async fn get_records(&self) -> Result<Vec<Record>>;
    async fn get(&self, record_id: &RecordId) -> Result<Option<Record>>;
    async fn update(&mut self, record: Record) -> UpdateResult<()>;
    async fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()>;
    async fn add_record(&mut self, record: Record) -> AddResult<()>;
    async fn dump(&self) -> Result<Vec<u8>>;
}

#[async_trait]
pub trait OpenRepository<T>
where
    T: RecordsRepository,
{
    async fn open(self, passwd: String) -> OpenResult<T>;
}

#[async_trait]
pub trait RepositoriesSource<T>: Debug + Clone + Sync + Send + 'static
where
    T: RecordsRepository,
{
    async fn create_repository(
        &self,
        repository_name: &str,
        passwd: String,
    ) -> CreateRepositoryResult<T>;
    async fn open_repository(&self, repository_name: &str, passwd: String) -> OpenResult<T>;
}
