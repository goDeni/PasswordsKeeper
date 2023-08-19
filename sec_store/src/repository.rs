use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::{
    cipher::EncryptionKey,
    record::{Record, RecordId},
};

struct RecordsRepository {
    file: PathBuf,
    records: Vec<Record>,
    passwd: EncryptionKey,
    identifier: String,
}

impl RecordsRepository {
    pub fn new(file: &Path, passwd: EncryptionKey) -> RecordsRepository {
        RecordsRepository {
            file: file.to_path_buf(),
            records: Vec::new(),
            passwd: passwd,
            identifier: Uuid::new_v4().to_string(),
        }
    }

    pub fn open(file: &Path, passwd: EncryptionKey) -> RecordsRepository {
        unimplemented!()
    }
    pub fn save(&self) {
        unimplemented!()
    }
    pub fn get_records() -> Vec<Record> {
        unimplemented!()
    }
    pub fn get(&mut self, record_id: RecordId) -> Record {
        unimplemented!()
    }
    pub fn delete(&mut self, record_id: RecordId) {
        unimplemented!()
    }
    pub fn add_record(&mut self, record: Record) {
        unimplemented!()
    }
}
