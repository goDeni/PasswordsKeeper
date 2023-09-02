pub mod file;

use anyhow::Result;

use sec_store::{
    cipher::EncryptionKey,
    repository::{OpenResult, RecordsRepository},
};

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryAlreadyExist;
pub type InitRepoResult<T> = Result<T, RepositoryAlreadyExist>;

#[derive(Debug, Clone, PartialEq)]
pub enum GetReposityError {
    WrongPassword,
    UnexpectedError,
}

pub type UserId = &'static str;
pub trait RepositoriesFactory {
    fn user_has_repository(&self, user_id: UserId) -> bool;
    fn get_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> OpenResult<Box<dyn RecordsRepository>>;
    fn initialize_user_repository(
        &self,
        user_id: UserId,
        passwd: EncryptionKey,
    ) -> InitRepoResult<Box<dyn RecordsRepository>>;
}
