pub mod file;

use anyhow::Result;

use sec_store::repository::OpenResult;

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryAlreadyExist;
pub type InitRepoResult<T> = Result<T, RepositoryAlreadyExist>;

#[derive(Debug, Clone, PartialEq)]
pub enum GetReposityError {
    WrongPassword,
    UnexpectedError,
}

pub type UserId = String;
pub trait RepositoriesFactory<T>: Clone + Sync + Send + 'static {
    fn user_has_repository(&self, user_id: &UserId) -> bool;
    fn get_user_repository(&self, user_id: &UserId, passwd: String) -> OpenResult<T>;
    fn initialize_user_repository(&self, user_id: &UserId, passwd: String) -> InitRepoResult<T>;
}
