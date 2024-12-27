pub mod file;

use std::fmt::{Debug, Display};

use crate::record::{Record, RecordId};
use anyhow::{Error, Result};
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
pub enum RepositoryOpenError {
    WrongPassword,
    DoesntExist,
    OpenError(anyhow::Error),
}
pub type OpenResult<T> = Result<T, RepositoryOpenError>;

impl Display for RepositoryOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryOpenError::WrongPassword => write!(f, "WrongPassword"),
            RepositoryOpenError::DoesntExist => write!(f, "DoesntExist"),
            RepositoryOpenError::OpenError(err) => {
                write!(f, "OpenError({}, {})", err, err.root_cause())
            }
        }
    }
}

pub trait RecordsRepository: Debug + Clone + Sync + Send + 'static {
    fn cancel(&mut self) -> Result<()>;
    fn save(&mut self) -> Result<()>;
    fn get_records(&self) -> Result<Vec<&Record>>;
    fn get(&mut self, record_id: &RecordId) -> Result<Option<&Record>>;
    fn update(&mut self, record: Record) -> UpdateResult<()>;
    fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()>;
    fn add_record(&mut self, record: Record) -> AddResult<()>;
    fn dump(&self) -> Result<Vec<u8>>;
}

pub trait OpenRepository<T>
where
    T: RecordsRepository,
{
    fn open(self, passwd: String) -> OpenResult<T>;
}
