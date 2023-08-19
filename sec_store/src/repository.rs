use std::fs::File;
use std::{collections::HashMap, path::PathBuf};

use uuid::Uuid;

use crate::cipher::{decrypt_string, encrypt_string, EncryptedData};
use crate::record::EncryptedRecord;
use crate::{
    cipher::{EncryptionKey, Result as ChipherResult},
    record::{Record, RecordId},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct RecordDoesntExist;
pub type UpdateResult<T> = std::result::Result<T, RecordDoesntExist>;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordAlreadyExist;
pub type AddResult<T> = std::result::Result<T, RecordAlreadyExist>;

type RepositoryId = String;
type RecordsMap = HashMap<RecordId, Record>;
pub struct RecordsRepository {
    pub identifier: RepositoryId,
    file: PathBuf,
    passwd: EncryptionKey,
    records: RecordsMap,
}

#[derive(Serialize, Deserialize)]
struct RawRepositoryJson(EncryptedData, Vec<EncryptedRecord>);

impl RecordsRepository {
    pub fn new(file: PathBuf, passwd: EncryptionKey) -> RecordsRepository {
        RecordsRepository {
            file: file,
            records: HashMap::new(),
            passwd: passwd,
            identifier: Uuid::new_v4().to_string(),
        }
    }

    pub fn open(file: PathBuf, passwd: EncryptionKey) -> ChipherResult<RecordsRepository> {
        let raw_rep =
            serde_json::from_reader::<File, RawRepositoryJson>(File::open(file.clone()).unwrap())
                .unwrap();

        let identifier = decrypt_string(passwd, raw_rep.0)?;
        let records = HashMap::from_iter(
            raw_rep
                .1
                .iter()
                .map(|encrypted_record| Record::decrypt(passwd, encrypted_record).unwrap())
                .map(|record| (record.id.clone(), record)),
        );

        Ok(RecordsRepository {
            file: file,
            identifier: identifier,
            passwd: passwd,
            records: records,
        })
    }
    pub fn save(&self) -> std::io::Result<PathBuf> {
        serde_json::to_writer(
            File::options()
                .create_new(true)
                .write(true)
                .open(&self.file)?,
            &RawRepositoryJson(
                encrypt_string(self.passwd, self.identifier.clone()),
                self.records
                    .values()
                    .map(|rec| rec.encrypt(self.passwd))
                    .collect::<Vec<EncryptedRecord>>(),
            ),
        )?;

        Ok(self.file.clone())
    }

    pub fn get_records(&self) -> Vec<&Record> {
        self.records.values().collect()
    }

    pub fn get(&mut self, record_id: &RecordId) -> Option<&Record> {
        self.records.get(record_id)
    }

    pub fn update(&mut self, record: Record) -> UpdateResult<()> {
        self.delete(&record.id)?;
        self.add_record(record).unwrap();

        Ok(())
    }

    pub fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()> {
        if self.get(record_id).is_none() {
            return Err(RecordDoesntExist);
        }
        self.records.remove(record_id);
        Ok(())
    }

    pub fn add_record(&mut self, record: Record) -> AddResult<()> {
        if self.get(&record.id).is_some() {
            return Err(RecordAlreadyExist);
        }
        self.records.insert(record.id.clone(), record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use crate::{record::Record, repository::RecordDoesntExist};

    use super::RecordsRepository;

    #[test]
    fn test_repository_add_record() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let file = tmp_dir.path().join("repo_file");

        let fields = vec![
            (String::from("Login"), String::from("1")),
            (String::from("Password"), String::from("2")),
        ];

        let passwd = "Passwd";
        let record = Record::new(fields);
        let mut repo = RecordsRepository::new(file, passwd);

        repo.add_record(record.clone()).unwrap();

        assert_eq!(repo.get(&record.id).unwrap(), &record);
        assert_eq!(repo.get_records(), vec![&record]);

        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_saving() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let file = tmp_dir.path().join("repo_file");

        let fields = vec![
            (String::from("Login"), String::from("1")),
            (String::from("Password"), String::from("2")),
        ];
        let record = Record::new(fields);

        let passwd = "Passwd";
        let mut repo = RecordsRepository::new(file.clone(), passwd);
        repo.add_record(record).unwrap();

        assert_eq!(file, repo.save().unwrap());

        let new_repo = RecordsRepository::open(file.clone(), passwd).unwrap();

        assert_eq!(new_repo.get_records(), repo.get_records());
        assert_eq!(new_repo.identifier, repo.identifier);

        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_update_record() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let file = tmp_dir.path().join("repo_file");

        let fields = vec![
            (String::from("Login"), String::from("1")),
            (String::from("Password"), String::from("2")),
        ];

        let passwd = "Passwd";

        let old_record = Record::new(fields);
        let mut repo = RecordsRepository::new(file, passwd);

        repo.add_record(old_record.clone()).unwrap();

        let mut new_record = repo.get(&old_record.id).unwrap().clone();
        new_record
            .fields
            .push(("Field3".to_string(), "3".to_string()));

        repo.update(new_record.clone()).unwrap();

        assert_eq!(&new_record, repo.get(&new_record.id).unwrap());
        assert_ne!(&old_record, repo.get(&new_record.id).unwrap());

        assert_eq!(repo.get_records(), vec![&new_record]);

        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_delete_record() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let file = tmp_dir.path().join("repo_file");

        let fields = vec![
            (String::from("Login"), String::from("1")),
            (String::from("Password"), String::from("2")),
        ];

        let passwd = "Passwd";

        let record = Record::new(fields);
        let mut repo = RecordsRepository::new(file, passwd);

        repo.add_record(record.clone()).unwrap();
        repo.delete(&record.id).unwrap();

        assert!(repo.get(&record.id).is_none());
        assert!(repo.get_records().is_empty());

        assert_eq!(repo.update(record.clone()), Err(RecordDoesntExist));
        assert_eq!(repo.delete(&record.id), Err(RecordDoesntExist));

        tmp_dir.close().unwrap();
    }
}
