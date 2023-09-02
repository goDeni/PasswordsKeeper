use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use sec_store::{
    cipher::EncryptionKey,
    repository::{OpenResult, RecordsRepository},
};

use crate::user_repo_factory::{InitRepoResult, RepositoriesFactory};

type Repo = Arc<Mutex<Box<dyn RecordsRepository>>>;
pub struct RespsitoriesStore {
    factory: Box<dyn RepositoriesFactory>,
    repos: HashMap<String, Repo>,
}


impl RespsitoriesStore {
    pub fn new(factory: Box<dyn RepositoriesFactory>) -> RespsitoriesStore {
        RespsitoriesStore {
            factory,
            repos: HashMap::new(),
        }
    }

    pub fn init_repo(&mut self, user_id: &String, passwd: EncryptionKey) -> InitRepoResult<Repo> {
        match self.get_repo(user_id) {
            Some(repo) => Ok(repo),
            None => {
                self.repos.insert(
                    user_id.clone(),
                    Arc::new(Mutex::new(
                        self.factory.initialize_user_repository(&user_id.to_string(), passwd)?,
                    )),
                );
                Ok(self.get_repo(user_id).unwrap())
            }
        }
    }

    pub fn open_repo(&mut self, user_id: &String, passwd: EncryptionKey) -> OpenResult<Repo> {
        match self.get_repo(user_id) {
            Some(repo) => Ok(repo),
            None => self
                .factory
                .get_user_repository(&user_id.to_string(), passwd)
                .map(|rep| {
                    self.repos.insert(user_id.clone(), Arc::new(Mutex::new(rep)));
                    self.get_repo(&user_id).unwrap()
                }),
        }
    }

    pub fn get_repo(&self, user_id: &String) -> Option<Repo> {
        self.repos.get(user_id).map(|rep| rep.clone())
    }

    pub fn close_repo(&mut self, user_id: &String) {
        self.repos.remove(user_id);
    }
}

#[cfg(test)]
mod tests {
    use sec_store::record::Record;
    use teloxide::types::UserId;
    use tempdir::TempDir;

    use crate::user_repo_factory::file::FileRepositoriesFactory;

    use super::RespsitoriesStore;

    #[test]
    fn test_store_init_and_get_repo() {
        let tmp_dir = TempDir::new("tests_").unwrap();
        let user_id = "123".to_string();
        let passwd = "passwd";

        let mut store =
            RespsitoriesStore::new(Box::new(FileRepositoriesFactory(tmp_dir.into_path())));

        let repo_lock = store.init_repo(&user_id, passwd).unwrap();
        let mut repo = repo_lock.lock().unwrap();

        let fields = vec![("Field1".to_string(), "v1".to_string())];

        repo.add_record(Record::new(fields.clone())).unwrap();
        let records = repo
            .get_records()
            .iter()
            .map(|&rec| rec.clone())
            .collect::<Vec<Record>>();

        drop(repo);
        drop(repo_lock);

        let repo_lock = store.get_repo(&user_id).unwrap();
        let repo = repo_lock.lock().unwrap();

        assert!(repo.get_records().len() > 0);
        assert_eq!(
            records,
            repo.get_records()
                .iter()
                .map(|&rec| rec.clone())
                .collect::<Vec<Record>>()
        );
    }

    #[test]
    fn test_store_close_repo() {
        let tmp_dir = TempDir::new("tests_").unwrap();
        let user_id = "123".to_string();
        let passwd = "passwd";

        let mut store =
            RespsitoriesStore::new(Box::new(FileRepositoriesFactory(tmp_dir.into_path())));

        store.init_repo(&user_id, passwd).unwrap();
        store.close_repo(&user_id);

        assert!(store.get_repo(&user_id).is_none());
    }

    #[test]
    fn test_open_repo() {
        let tmp_dir = TempDir::new("tests_").unwrap();
        let user_id = "123".to_string();
        let passwd = "passwd";

        let mut store =
            RespsitoriesStore::new(Box::new(FileRepositoriesFactory(tmp_dir.into_path())));

        let repo_lock = store.init_repo(&user_id, passwd).unwrap();
        let mut repo = repo_lock.lock().unwrap();

        let fields = vec![("Field1".to_string(), "v1".to_string())];

        repo.add_record(Record::new(fields.clone())).unwrap();
        let records = repo
            .get_records()
            .iter()
            .map(|&rec| rec.clone())
            .collect::<Vec<Record>>();

        repo.save().unwrap();

        drop(repo);
        drop(repo_lock);

        store.close_repo(&user_id);

        let repo_lock = store.open_repo(&user_id, passwd).unwrap();
        let repo = repo_lock.lock().unwrap();

        assert!(repo.get_records().len() > 0);
        assert_eq!(
            records,
            repo.get_records()
                .iter()
                .map(|&rec| rec.clone())
                .collect::<Vec<Record>>()
        );
    }
}
