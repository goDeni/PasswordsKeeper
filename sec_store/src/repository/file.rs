use std::fs::File;
use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::cipher::{decrypt_string, encrypt_string, DecryptionError, EncryptedData};
use crate::record::EncryptedRecord;
use crate::repository::{
    AddResult, OpenRepository, OpenResult, RecordAlreadyExist, RecordDoesntExist,
    RecordsRepository, RepositoryOpenError, UpdateResult,
};
use crate::{
    cipher::EncryptionKey,
    record::{Record, RecordId},
};
use serde::{Deserialize, Serialize};

type RepositoryId = String;
type RecordsMap = HashMap<RecordId, Record>;

#[derive(Debug)]
pub struct RecordsFileRepository {
    pub identifier: RepositoryId,
    file: PathBuf,
    passwd: EncryptionKey,
    records: RecordsMap,
}
pub struct OpenRecordsFileRepository(pub PathBuf);

#[derive(Serialize, Deserialize)]
struct RawRepositoryJson(EncryptedData, Vec<EncryptedRecord>);

impl RecordsFileRepository {
    pub fn new(file: PathBuf, passwd: EncryptionKey) -> RecordsFileRepository {
        RecordsFileRepository {
            file: file,
            records: HashMap::new(),
            passwd: passwd,
            identifier: Uuid::new_v4().to_string(),
        }
    }
}

impl OpenRepository<RecordsFileRepository> for OpenRecordsFileRepository {
    fn open(self, passwd: EncryptionKey) -> OpenResult<RecordsFileRepository> {
        if !self.0.exists() {
            return Err(RepositoryOpenError::DoesntExist);
        }

        match File::open(self.0.clone()) {
            Err(_) => Err(RepositoryOpenError::UnexpectedError),
            Ok(file) => {
                let raw_rep = serde_json::from_reader::<File, RawRepositoryJson>(file).unwrap();

                match decrypt_string(&passwd, raw_rep.0) {
                    Err(DecryptionError::WrongPassword) => Err(RepositoryOpenError::WrongPassword),
                    Err(DecryptionError::UnexpectedError) => {
                        Err(RepositoryOpenError::UnexpectedError)
                    }
                    Ok(identifier) => Ok(RecordsFileRepository {
                        file: self.0,
                        identifier: identifier,
                        records: HashMap::from_iter(
                            raw_rep
                                .1
                                .iter()
                                .map(|encrypted_record| {
                                    Record::decrypt(&passwd, encrypted_record).unwrap()
                                })
                                .map(|record| (record.id.clone(), record)),
                        ),
                        passwd: passwd,
                    }),
                }
            }
        }
    }
}

impl RecordsRepository for RecordsFileRepository {
    fn save(&self) -> Result<()> {
        serde_json::to_writer(
            File::options()
                .create_new(true)
                .write(true)
                .open(&self.file)
                .with_context(|| format!("Failed file open: {:?}", self.file))?,
            &RawRepositoryJson(
                encrypt_string(&self.passwd, self.identifier.clone()),
                self.records
                    .values()
                    .map(|rec| rec.encrypt(&self.passwd))
                    .collect::<Vec<EncryptedRecord>>(),
            ),
        )
        .with_context(|| format!("Failed json writing {:?}", self.file))
    }

    fn get_records(&self) -> Vec<&Record> {
        self.records.values().collect()
    }

    fn get(&mut self, record_id: &RecordId) -> Option<&Record> {
        self.records.get(record_id)
    }

    fn update(&mut self, record: Record) -> UpdateResult<()> {
        self.delete(&record.id)?;
        self.add_record(record).unwrap();

        Ok(())
    }

    fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()> {
        if self.get(record_id).is_none() {
            return Err(RecordDoesntExist);
        }
        self.records.remove(record_id);
        Ok(())
    }

    fn add_record(&mut self, record: Record) -> AddResult<()> {
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

    use crate::{
        record::Record,
        repository::file::{OpenRecordsFileRepository, RecordDoesntExist, RepositoryOpenError},
        repository::{OpenRepository, RecordsRepository},
    };

    use super::RecordsFileRepository;

    #[test]
    fn test_repository_add_record() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let file = tmp_dir.path().join("repo_file");

        let fields = vec![
            (String::from("Login"), String::from("1")),
            (String::from("Password"), String::from("2")),
        ];

        let passwd = "Passwd".to_string();
        let record = Record::new(fields);
        let mut repo = RecordsFileRepository::new(file, passwd);

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

        let passwd = "Passwd".to_string();
        let mut repo = RecordsFileRepository::new(file.clone(), passwd.clone());
        repo.add_record(record).unwrap();
        repo.save().unwrap();

        let new_repo = OpenRecordsFileRepository(file.clone())
            .open(passwd)
            .unwrap();

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

        let passwd = "Passwd".to_string();

        let old_record = Record::new(fields);
        let mut repo = RecordsFileRepository::new(file, passwd);

        repo.add_record(old_record.clone()).unwrap();

        let mut new_record = repo.get(&old_record.id).unwrap().clone();
        new_record
            .add_field("Field3".to_string(), "3".to_string())
            .unwrap();

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

        let passwd = "Passwd".to_string();

        let record = Record::new(fields);
        let mut repo = RecordsFileRepository::new(file, passwd);

        repo.add_record(record.clone()).unwrap();
        repo.delete(&record.id).unwrap();

        assert!(repo.get(&record.id).is_none());
        assert!(repo.get_records().is_empty());

        assert_eq!(repo.update(record.clone()), Err(RecordDoesntExist));
        assert_eq!(repo.delete(&record.id), Err(RecordDoesntExist));

        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_open_with_wrong_passwd() {
        let tmp_dir = TempDir::new("test_").unwrap();

        let repo = RecordsFileRepository::new(
            tmp_dir.path().join("repo_file"),
            "One password".to_string(),
        );
        repo.save().unwrap();

        let result = OpenRecordsFileRepository(repo.file)
            .open("Wrong passwd".to_string())
            .unwrap_err();
        assert_eq!(result, RepositoryOpenError::WrongPassword);

        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_open_missed_file() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let result = OpenRecordsFileRepository(tmp_dir.path().join("any_file"))
            .open("Wrong passwd".to_string())
            .unwrap_err();

        assert_eq!(result, RepositoryOpenError::DoesntExist);

        tmp_dir.close().unwrap();
    }
}
