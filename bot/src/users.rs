use std::path::PathBuf;

use sec_store::{
    cipher::EncryptionKey,
    repository::{OpenResult, RecordsRepository},
};

type UserId = &'static str;

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryAlreadyExist;
pub type InitRepoResult<T> = std::result::Result<T, RepositoryAlreadyExist>;

struct RepositoriesFactory(PathBuf);
impl RepositoriesFactory {
    fn get_repository_path(&self, user_id: UserId) -> PathBuf {
        self.0.join(format!("rep_{}", user_id))
    }
    pub fn user_has_repository(&self, user_id: UserId) -> bool {
        self.get_repository_path(user_id).exists()
    }
    pub fn get_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> OpenResult<RecordsRepository> {
        RecordsRepository::open(self.get_repository_path(&user_id), passwd)
    }
    pub fn initialize_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> InitRepoResult<RecordsRepository> {
        match self.user_has_repository(&user_id) {
            true => Err(RepositoryAlreadyExist),
            false => Ok(RecordsRepository::new(
                self.get_repository_path(user_id),
                passwd,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use sec_store::repository::RepositoryOpenError;
    use tempdir::TempDir;

    use super::RepositoriesFactory;

    #[test]
    fn test_repo_not_exist() {
        let tmp_dir = TempDir::new("tests_").unwrap();

        let user_id = "user_id";
        let passwd = "123";

        let factory = RepositoriesFactory(tmp_dir.into_path());

        assert!(!factory.user_has_repository(user_id));
        let result = factory.get_user_repository(user_id, passwd).unwrap_err();

        assert_eq!(result, RepositoryOpenError::FileAccessError);
    }

    #[test]
    fn test_repo_initialization() {
        let tmp_dir = TempDir::new("tests_").unwrap();

        let user_id = "user_id";
        let passwd = "123";

        let factory = RepositoriesFactory(tmp_dir.into_path());

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

        let factory = RepositoriesFactory(tmp_dir.into_path());

        factory
            .initialize_user_repository(user_id, passwd)
            .unwrap()
            .save()
            .unwrap();
        let result = factory.get_user_repository(user_id, "312").unwrap_err();

        assert_eq!(result, RepositoryOpenError::WrongPassword);
    }
}
