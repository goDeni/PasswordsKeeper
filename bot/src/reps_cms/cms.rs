use std::{collections::HashMap, sync::Arc};

use sec_store::{
    cipher::EncryptionKey,
    repository::{OpenResult, RecordsRepository},
};

use crate::user_repo_factory::{InitRepoResult, RepositoriesFactory, UserId};

pub struct RespsitoriesStore {
    factory: Box<dyn RepositoriesFactory>,
    repos: HashMap<UserId, Arc<Box<dyn RecordsRepository>>>,
}

impl RespsitoriesStore {
    pub fn new(factory: Box<dyn RepositoriesFactory>) -> RespsitoriesStore {
        RespsitoriesStore {
            factory: factory,
            repos: HashMap::new(),
        }
    }

    pub fn init_repo(
        &mut self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> InitRepoResult<Arc<Box<dyn RecordsRepository>>> {
        match self.get_repo(user_id) {
            Some(repo) => Ok(repo),
            None => {
                self.repos.insert(
                    user_id,
                    Arc::new(self.factory.initialize_user_repository(user_id, passwd)?),
                );
                Ok(self.get_repo(user_id).unwrap())
            }
        }
    }

    pub fn open_repo(
        &mut self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> OpenResult<Arc<Box<dyn RecordsRepository>>> {
        match self.get_repo(user_id) {
            Some(repo) => Ok(repo),
            None => self
                .factory
                .get_user_repository(user_id, passwd)
                .map(|rep| {
                    self.repos.insert(user_id, Arc::new(rep));
                    self.get_repo(user_id).unwrap()
                }),
        }
    }

    pub fn get_repo(&self, user_id: UserId) -> Option<Arc<Box<dyn RecordsRepository>>> {
        self.repos.get(user_id).map(|rep| rep.clone())
    }

    pub fn close_repo(&mut self, user_id: UserId) {
        self.repos.remove(user_id);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sec_store::record::Record;
    use tempdir::TempDir;

    use crate::user_repo_factory::file::FileRepositoriesFactory;

    use super::RespsitoriesStore;

    #[test]
    fn test_one() {
        let tmp_dir = TempDir::new("tests_").unwrap();
        let user_id = "user_id";
        let passwd = "passwd";

        // let mut store = RespsitoriesStore::new(Box::new(FileRepositoriesFactory(tmp_dir.into_path())));

        // let mut repo = store.init_repo(user_id, passwd).unwrap();
        // let rep = Arc::get_mut(&mut repo);

        // rep.add_record(Record::new(vec![("Field1".to_string(), "v1".to_string())]));
    }
}
