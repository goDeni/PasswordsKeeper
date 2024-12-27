use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use tempfile::NamedTempFile;
use uuid::Uuid;

use super::{AddRecordError, UpdateRecordError};
use crate::cipher::{decrypt_string, encrypt_string, DecryptionError, EncryptedData};
use crate::record::EncryptedRecord;
use crate::record::{Record, RecordId};
use crate::repository::{
    AddResult, OpenRepository, OpenResult, RecordsRepository, RepositoryOpenError, UpdateResult,
};
use serde::{Deserialize, Serialize};
use std::io::prelude::Read;

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryId(Arc<str>);
impl From<String> for RepositoryId {
    fn from(value: String) -> Self {
        RepositoryId(value.into())
    }
}
impl RepositoryId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

type RecordsMap = HashMap<RecordId, Record>;

#[derive(Debug, Clone)]
pub struct RecordsFileRepository {
    pub identifier: RepositoryId,
    file: PathBuf,
    passwd: String,
    records: RecordsMap,
    saved_records: RecordsMap,
}
pub struct OpenRecordsFileRepository(pub PathBuf);

#[derive(Serialize, Deserialize)]
struct RawRepositoryJson(EncryptedData, Vec<EncryptedRecord>);

impl RecordsFileRepository {
    pub fn new(file: PathBuf, passwd: String) -> RecordsFileRepository {
        RecordsFileRepository {
            file,
            records: HashMap::new(),
            passwd,
            identifier: Uuid::new_v4().to_string().into(),
            saved_records: HashMap::new(),
        }
    }
}

impl OpenRepository<RecordsFileRepository> for OpenRecordsFileRepository {
    fn open(self, passwd: String) -> OpenResult<RecordsFileRepository> {
        if !self.0.exists() {
            return Err(RepositoryOpenError::DoesntExist);
        }

        match File::open(self.0.clone())
            .with_context(|| format!("Failed file open {:?}", self.0.to_str()))
        {
            Err(err) => Err(RepositoryOpenError::OpenError(err)),
            Ok(file) => {
                let raw_rep = serde_json::from_reader::<File, RawRepositoryJson>(file)
                    .with_context(|| format!("Failed file {:?} deserealisation", self.0.to_str()))
                    .map_err(RepositoryOpenError::OpenError)?;

                match decrypt_string(&passwd, raw_rep.0) {
                    Err(DecryptionError::WrongPassword) => Err(RepositoryOpenError::WrongPassword),
                    Err(DecryptionError::EncodingError(err)) => {
                        Err(RepositoryOpenError::OpenError(anyhow!(
                            "Got encoding error \"{}\" for file \"{:?}\"",
                            err,
                            self.0.to_str()
                        )))
                    }
                    Ok(identifier) => {
                        let records = HashMap::from_iter(
                            raw_rep
                                .1
                                .iter()
                                .map(|encrypted_record| {
                                    Record::decrypt(&passwd, encrypted_record).unwrap()
                                })
                                .map(|record| (record.id.clone(), record)),
                        );
                        Ok(RecordsFileRepository {
                            file: self.0,
                            identifier: identifier.into(),
                            passwd,
                            records: records.clone(),
                            saved_records: records,
                        })
                    }
                }
            }
        }
    }
}

impl RecordsRepository for RecordsFileRepository {
    fn cancel(&mut self) -> Result<()> {
        self.records = self.saved_records.clone();
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        let mut tmp_file = NamedTempFile::new_in(
            self.file
                .parent()
                .with_context(|| format!("Failed get parent directory for {:?}", self.file))?,
        )?;
        serde_json::to_writer(
            &tmp_file,
            &RawRepositoryJson(
                encrypt_string(&self.passwd, self.identifier.as_str()),
                self.records
                    .values()
                    .map(|rec| rec.encrypt(&self.passwd))
                    .collect::<Vec<EncryptedRecord>>(),
            ),
        )
        .with_context(|| format!("Failed json writing {:?}", self.file))?;

        tmp_file.flush()?;
        tmp_file.persist(self.file.as_path())?;

        self.saved_records = self.records.clone();

        Ok(())
    }

    fn get_records(&self) -> Result<Vec<&Record>> {
        Ok(self.records.values().collect())
    }

    fn get(&mut self, record_id: &RecordId) -> Result<Option<&Record>> {
        Ok(self.records.get(record_id))
    }

    fn update(&mut self, record: Record) -> UpdateResult<()> {
        self.delete(&record.id)?;
        self.add_record(record).unwrap();

        Ok(())
    }

    fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()> {
        if self
            .get(record_id)
            .map_err(UpdateRecordError::UnxpectedError)?
            .is_none()
        {
            return Err(UpdateRecordError::RecordDoesntExist);
        }
        self.records.remove(record_id);
        Ok(())
    }

    fn add_record(&mut self, record: Record) -> AddResult<()> {
        if self
            .get(&record.id)
            .map_err(AddRecordError::UnxpectedError)?
            .is_some()
        {
            return Err(AddRecordError::RecordDoesntExist);
        }
        self.records.insert(record.id.clone(), record);
        Ok(())
    }
    fn dump(&self) -> Result<Vec<u8>> {
        let mut buff = Vec::new();
        File::open(&self.file)?.read_to_end(&mut buff)?;

        Ok(buff)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::prelude::Write;
    use tempdir::TempDir;

    use crate::{
        record::Record,
        repository::{
            file::{OpenRecordsFileRepository, RepositoryOpenError},
            UpdateRecordError,
        },
        repository::{OpenRepository, RecordsRepository},
    };
    use anyhow::Result;

    use super::RecordsFileRepository;

    #[test]
    fn test_repository_add_record() -> Result<()> {
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

        assert_eq!(repo.get(&record.id)?.unwrap(), &record);
        assert_eq!(repo.get_records()?, vec![&record]);

        tmp_dir.close().unwrap();

        Ok(())
    }

    #[test]
    fn test_repository_saving() -> Result<()> {
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

        assert_eq!(new_repo.get_records()?, repo.get_records()?);
        assert_eq!(new_repo.identifier, repo.identifier);

        tmp_dir.close().unwrap();

        Ok(())
    }

    #[test]
    fn test_repository_update_record() -> Result<()> {
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

        let mut new_record = repo.get(&old_record.id)?.unwrap().clone();
        new_record
            .add_field("Field3".to_string(), "3".to_string())
            .unwrap();

        repo.update(new_record.clone()).unwrap();

        assert_eq!(&new_record, repo.get(&new_record.id)?.unwrap());
        assert_ne!(&old_record, repo.get(&new_record.id)?.unwrap());

        assert_eq!(repo.get_records()?, vec![&new_record]);

        tmp_dir.close().unwrap();

        Ok(())
    }

    #[test]
    fn test_repository_delete_record() -> Result<()> {
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

        assert!(repo.get(&record.id)?.is_none());
        assert!(repo.get_records()?.is_empty());

        assert!(matches!(
            repo.update(record.clone()).unwrap_err(),
            UpdateRecordError::RecordDoesntExist
        ));
        assert!(matches!(
            repo.delete(&record.id).unwrap_err(),
            UpdateRecordError::RecordDoesntExist
        ));

        tmp_dir.close().unwrap();

        Ok(())
    }

    #[test]
    fn test_repository_open_with_wrong_passwd() {
        let tmp_dir = TempDir::new("test_").unwrap();

        let mut repo = RecordsFileRepository::new(
            tmp_dir.path().join("repo_file"),
            "One password".to_string(),
        );
        repo.save().unwrap();

        let result = OpenRecordsFileRepository(repo.file)
            .open("Wrong passwd".to_string())
            .unwrap_err();

        assert!(matches!(result, RepositoryOpenError::WrongPassword));
        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_open_missed_file() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let result = OpenRecordsFileRepository(tmp_dir.path().join("any_file"))
            .open("Wrong passwd".to_string())
            .unwrap_err();

        assert!(matches!(result, RepositoryOpenError::DoesntExist));
        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_dump() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let pass = "One password".to_string();

        let mut repo = RecordsFileRepository::new(tmp_dir.path().join("repo_file"), pass);
        repo.save().unwrap();

        let dump_res = repo.dump().unwrap();
        assert!(!dump_res.is_empty());

        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_repository_load_dumped() {
        let tmp_dir = TempDir::new("test_").unwrap();
        let pass = "One password".to_string();

        let mut repo = RecordsFileRepository::new(tmp_dir.path().join("repo_file"), pass.clone());

        repo.add_record(Record::new(vec![
            (String::from("Login"), String::from("1")),
            (String::from("Password"), String::from("2")),
        ]))
        .unwrap();
        repo.add_record(Record::new(vec![
            (String::from("Login"), String::from("3")),
            (String::from("Password"), String::from("4")),
        ]))
        .unwrap();

        repo.save().unwrap();

        let dump_res = repo.dump().unwrap();

        let loaded_repo_path = tmp_dir.path().join("repo_file_loads");
        File::create(&loaded_repo_path)
            .unwrap()
            .write_all(&dump_res)
            .unwrap();

        let loaded_repo = OpenRecordsFileRepository(loaded_repo_path)
            .open(pass)
            .unwrap();

        let mut expected_records = repo.get_records().unwrap();
        let mut real_records = loaded_repo.get_records().unwrap();

        expected_records.sort_by(|a, b| a.id.cmp(&b.id));
        real_records.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(expected_records, real_records);

        tmp_dir.close().unwrap();
    }
}
