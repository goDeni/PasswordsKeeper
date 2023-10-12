pub mod file;

use crate::{
    cipher::EncryptionKey,
    record::{Record, RecordId},
};
use anyhow::Result;
use std::{
    fmt::{Debug, Display},
    result::Result as StdResult,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub struct RecordDoesntExist;
pub type UpdateResult<T> = StdResult<T, RecordDoesntExist>;

impl Display for RecordDoesntExist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RecordDoesntExist")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordAlreadyExist;
pub type AddResult<T> = StdResult<T, RecordAlreadyExist>;

pub type OpenResult<T> = Result<T, RepositoryOpenError>;

#[derive(Debug, Error)]
pub enum RepositoryOpenError {
    WrongPassword,
    DoesntExist,
    OpenError(anyhow::Error),
}

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
    fn get_records(&self) -> Vec<&Record>;
    fn get(&mut self, record_id: &RecordId) -> Option<&Record>;
    fn update(&mut self, record: Record) -> UpdateResult<()>;
    fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()>;
    fn add_record(&mut self, record: Record) -> AddResult<()>;
}

pub trait OpenRepository<T>
where
    T: RecordsRepository,
{
    fn open(self, passwd: EncryptionKey) -> OpenResult<T>;
}
