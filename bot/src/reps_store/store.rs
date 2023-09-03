use std::{collections::HashMap, sync::Arc};

use async_mutex::Mutex;
use sec_store::{
    cipher::EncryptionKey,
    repository::{OpenResult, RecordsRepository},
};

use crate::user_repo_factory::{InitRepoResult, RepositoriesFactory};

pub struct RepositoriesStore<T, R> {
    factory: T,
    repos: HashMap<String, Arc<Mutex<R>>>,
}

impl<T, R> RepositoriesStore<T, R>
where
    T: RepositoriesFactory<R>,
    R: RecordsRepository,
{
    pub fn new(factory: T) -> Self {
        RepositoriesStore {
            factory,
            repos: HashMap::new(),
        }
    }

    pub fn exist(&self, user_id: &String) -> bool {
        self.factory.user_has_repository(user_id)
    }

    pub fn init(
        &mut self,
        user_id: &String,
        passwd: EncryptionKey,
    ) -> InitRepoResult<Arc<Mutex<R>>> {
        if !self.repos.contains_key(user_id) {
            let repo = self
                .factory
                .initialize_user_repository(&user_id.to_string(), passwd)?;
            self.repos
                .insert(user_id.clone(), Arc::new(Mutex::new(repo)));
        }

        Ok(self.get(user_id).unwrap())
    }

    pub fn open(&mut self, user_id: &String, passwd: EncryptionKey) -> OpenResult<Arc<Mutex<R>>> {
        if !self.repos.contains_key(user_id) {
            let repo = self.factory.get_user_repository(user_id, passwd).unwrap();
            self.repos
                .insert(user_id.clone(), Arc::new(Mutex::new(repo)));
        }

        Ok(self.get(user_id).unwrap())
    }

    pub fn get(&self, user_id: &String) -> Option<Arc<Mutex<R>>> {
        self.repos.get(user_id).map(|rep| Arc::clone(rep))
    }

    pub fn close(&mut self, user_id: &String) {
        self.repos.remove(user_id);
    }
}

#[cfg(test)]
mod tests {
    use sec_store::{record::Record, repository::RecordsRepository};
    use tempdir::TempDir;

    use crate::user_repo_factory::file::FileRepositoriesFactory;

    use super::RepositoriesStore;

    #[tokio::test]
    async fn test_store_init_and_get_repo() {
        let tmp_dir = TempDir::new("tests_").unwrap();
        let user_id = "123".to_string();
        let passwd = "passwd";

        let mut store = RepositoriesStore::new(FileRepositoriesFactory(tmp_dir.into_path()));

        let repo_lock = store.init(&user_id, passwd).unwrap();
        let mut repo = repo_lock.lock().await;

        let fields = vec![("Field1".to_string(), "v1".to_string())];

        repo.add_record(Record::new(fields.clone())).unwrap();
        let records = repo
            .get_records()
            .iter()
            .map(|&rec| rec.clone())
            .collect::<Vec<Record>>();

        drop(repo);
        drop(repo_lock);

        let repo_lock = store.get(&user_id).unwrap();
        let repo = repo_lock.lock().await;

        assert!(repo.get_records().len() > 0);
        assert_eq!(
            records,
            repo.get_records()
                .iter()
                .map(|&rec| rec.clone())
                .collect::<Vec<Record>>()
        );
    }

    #[tokio::test]
    async fn test_store_close_repo() {
        let tmp_dir = TempDir::new("tests_").unwrap();
        let user_id = "123".to_string();
        let passwd = "passwd";

        let mut store = RepositoriesStore::new(FileRepositoriesFactory(tmp_dir.into_path()));

        store.init(&user_id, passwd).unwrap();
        store.close(&user_id);

        assert!(store.get(&user_id).is_none());
    }

    #[tokio::test]
    async fn test_open_repo() {
        let tmp_dir = TempDir::new("tests_").unwrap();
        let user_id = "123".to_string();
        let passwd = "passwd";

        let mut store = RepositoriesStore::new(FileRepositoriesFactory(tmp_dir.into_path()));

        let repo_lock = store.init(&user_id, passwd).unwrap();
        let mut repo = repo_lock.lock().await;

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

        store.close(&user_id);

        let repo_lock = store.open(&user_id, passwd).unwrap();
        let repo = repo_lock.lock().await;

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
