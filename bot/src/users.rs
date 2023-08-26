use std::{
    fmt::format,
    path::{Path, PathBuf},
};

use sec_store::{cipher::EncryptionKey, repository::RecordsRepository};

type UserId = &'static str;

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryDoesntExist;
pub type GetRepoResult<T> = std::result::Result<T, RepositoryDoesntExist>;

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryAlreadyExist;
pub type InitRepoResult<T> = std::result::Result<T, RepositoryAlreadyExist>;

struct RepositoriesFabric(PathBuf);
impl RepositoriesFabric {
    fn get_repository_path(&self, user_id: UserId) -> PathBuf {
        self.0.join(format!("rep_{}", user_id))
    }
    pub fn user_has_repository(&self, user_id: &UserId) -> bool {
        self.get_repository_path(user_id).exists()
    }
    pub fn get_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> GetRepoResult<RecordsRepository> {
        match self.user_has_repository(&user_id) {
            true => {
                let path = self.get_repository_path(&user_id);
                let res = RecordsRepository::open(path, passwd);

                Ok(res.unwrap())
            }
            false => Err(RepositoryDoesntExist),
        }
    }
    pub fn initialize_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> InitRepoResult<RecordsRepository> {
        match self.user_has_repository(&user_id) {
            true => Err(RepositoryAlreadyExist),
            false => {
                let path = self.get_repository_path(&user_id);
                let repo = RecordsRepository::new(path, passwd);

                Ok(repo)
            }
        }
    }
}
