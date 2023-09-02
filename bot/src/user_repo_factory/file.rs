use std::path::PathBuf;

use sec_store::{
    cipher::EncryptionKey,
    repository::{
        file::{OpenRecordsFileRepository, RecordsFileRepository},
        RecordsRepository,
    },
    repository::{OpenRepository, OpenResult},
};

use crate::user_repo_factory::{InitRepoResult, RepositoriesFactory, RepositoryAlreadyExist};

type UserId = &'static str;

pub struct FileRepositoriesFactory(pub PathBuf);
impl FileRepositoriesFactory {
    fn get_repository_path(&self, user_id: UserId) -> PathBuf {
        self.0.join(format!("rep_{}", user_id))
    }
}

impl RepositoriesFactory for FileRepositoriesFactory {
    fn user_has_repository(&self, user_id: UserId) -> bool {
        self.get_repository_path(user_id).exists()
    }
    fn get_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> OpenResult<Box<dyn RecordsRepository>> {
        match OpenRecordsFileRepository(self.get_repository_path(&user_id)).open(passwd) {
            Ok(rep) => Ok(Box::new(rep)),
            Err(err) => Err(err),
        }
    }
    fn initialize_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> InitRepoResult<Box<dyn RecordsRepository>> {
        match self.user_has_repository(&user_id) {
            true => Err(RepositoryAlreadyExist),
            false => Ok(Box::new(RecordsFileRepository::new(
                self.get_repository_path(user_id),
                passwd,
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use sec_store::repository::{RecordsRepository, RepositoryOpenError};
    use tempdir::TempDir;

    use crate::user_repo_factory::{file::FileRepositoriesFactory, RepositoriesFactory};

    #[test]
    fn test_repo_not_exist() {
        let tmp_dir = TempDir::new("tests_").unwrap();

        let user_id = "user_id";
        let passwd = "123";

        let factory = FileRepositoriesFactory(tmp_dir.into_path());

        assert!(!factory.user_has_repository(user_id));
        assert!(factory.get_user_repository(user_id, passwd).is_err());
    }

    #[test]
    fn test_repo_initialization() {
        let tmp_dir = TempDir::new("tests_").unwrap();

        let user_id = "user_id";
        let passwd = "123";

        let factory = FileRepositoriesFactory(tmp_dir.into_path());

        let repo = factory.initialize_user_repository(user_id, passwd).unwrap();
        repo.save().unwrap();

        assert!(factory.user_has_repository(user_id));

        let result = factory.get_user_repository(user_id, passwd);
        assert!(result.is_ok());
    }

    #[test]
    fn test_repo_open_with_wrong_password() {
        let tmp_dir = TempDir::new("tests_").unwrap();

        let user_id = "user_id";
        let passwd = "123";

        let factory = FileRepositoriesFactory(tmp_dir.into_path());

        factory
            .initialize_user_repository(user_id, passwd)
            .unwrap()
            .save()
            .unwrap();
        let result = factory.get_user_repository(user_id, "312").unwrap_err();

        assert_eq!(result, RepositoryOpenError::WrongPassword);
    }
}
