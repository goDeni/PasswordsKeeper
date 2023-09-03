pub mod file;

use crate::{
    cipher::EncryptionKey,
    record::{Record, RecordId},
};
use anyhow::Result;
use std::result::Result as StdResult;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordDoesntExist;
pub type UpdateResult<T> = StdResult<T, RecordDoesntExist>;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordAlreadyExist;
pub type AddResult<T> = StdResult<T, RecordAlreadyExist>;

pub type OpenResult<T> = Result<T, RepositoryOpenError>;

#[derive(Debug, Clone, PartialEq)]
pub enum RepositoryOpenError {
    WrongPassword,
    DoesntExist,
    UnexpectedError,
}

pub trait RecordsRepository {
    fn save(&self) -> Result<()>;
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

impl core::fmt::Debug for dyn RecordsRepository {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "RecordsRepository")
    }
}
