use crate::record::{Record, RecordId};
use anyhow::Result;
use std::result::Result as StdResult;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordDoesntExist;
pub type UpdateResult<T> = StdResult<T, RecordDoesntExist>;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordAlreadyExist;
pub type AddResult<T> = StdResult<T, RecordAlreadyExist>;

pub trait RecordsRepository {
    fn save(&self) -> Result<()>;
    fn get_records(&self) -> Vec<&Record>;
    fn get(&mut self, record_id: &RecordId) -> Option<&Record>;
    fn update(&mut self, record: Record) -> UpdateResult<()>;
    fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()>;
    fn add_record(&mut self, record: Record) -> AddResult<()>;
}
