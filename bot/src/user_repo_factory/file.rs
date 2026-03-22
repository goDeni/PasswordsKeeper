use super::UserId;
use crate::user_repo_factory::{
    InitRepoResult, LoadResult, RepositoriesFactory, RepositoryAlreadyExist, RepositoryLoadError,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use sec_store::{
    repository::file::{OpenRecordsFileRepository, RecordsFileRepository},
    repository::{OpenRepository, OpenResult, RepositoryOpenError},
};
use std::fs::rename;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileRepositoriesFactory(pub PathBuf);

impl FileRepositoriesFactory {
    fn get_repository_path(&self, user_id: &UserId) -> PathBuf {
        self.0.join(format!("rep_{}", user_id))
    }
}

#[async_trait]
impl RepositoriesFactory<RecordsFileRepository> for FileRepositoriesFactory {
    fn user_has_repository(&self, user_id: &UserId) -> bool {
        self.get_repository_path(user_id).exists()
    }
    async fn get_user_repository(
        &self,
        user_id: &UserId,
        passwd: String,
    ) -> OpenResult<RecordsFileRepository> {
        OpenRecordsFileRepository(self.get_repository_path(user_id))
            .open(passwd)
            .await
    }
    async fn load_user_repository<P: AsRef<Path> + Send>(
        &self,
        user_id: &UserId,
        passwd: String,
        file: P,
    ) -> LoadResult<RecordsFileRepository> {
        match OpenRecordsFileRepository(file.as_ref().to_path_buf())
            .open(passwd.clone())
            .await
        {
            Ok(_) => {
                rename(file, self.get_repository_path(user_id))
                    .with_context(|| format!("Failed repository save for {}", user_id))
                    .map_err(RepositoryLoadError::UnexpectedError)?;

                match self.get_user_repository(user_id, passwd).await {
                    Ok(repo) => Ok(repo),
                    Err(err) => Err(RepositoryLoadError::UnexpectedError(anyhow!(format!(
                        "Failed repository open after it has been loaded: {:?}",
                        err
                    )))),
                }
            }
            Err(err) => Err(match err {
                RepositoryOpenError::OpenError(err) => RepositoryLoadError::OpenError(err),
                RepositoryOpenError::DoesntExist => RepositoryLoadError::UnexpectedError(anyhow!(
                    "Repository already doesn't exists?"
                )),
                RepositoryOpenError::InvalidRepositoryName(name) => {
                    RepositoryLoadError::UnexpectedError(anyhow!(
                        "Invalid repository name for user repository: {}",
                        name
                    ))
                }
                RepositoryOpenError::WrongPassword => RepositoryLoadError::WrongPassword,
            }),
        }
    }
    async fn initialize_user_repository(
        &self,
        user_id: &UserId,
        passwd: String,
    ) -> InitRepoResult<RecordsFileRepository> {
        match self.user_has_repository(user_id) {
            true => Err(RepositoryAlreadyExist),
            false => Ok(RecordsFileRepository::new(
                self.get_repository_path(user_id),
                passwd,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use sec_store::repository::{RecordsRepository, RepositoryOpenError};
    use tempfile::TempDir;

    use crate::user_repo_factory::{file::FileRepositoriesFactory, RepositoriesFactory};

    #[tokio::test]
    async fn test_repo_not_exist() {
        let tmp_dir = TempDir::new().unwrap();

        let user_id = "user_id".to_string();
        let passwd = "123".to_string();

        let factory = FileRepositoriesFactory(tmp_dir.keep());

        assert!(!factory.user_has_repository(&user_id));
        assert!(factory.get_user_repository(&user_id, passwd).await.is_err());
    }

    #[tokio::test]
    async fn test_repo_initialization() {
        let tmp_dir = TempDir::new().unwrap();

        let user_id = "user_id".to_string();
        let passwd = "123".to_string();

        let factory = FileRepositoriesFactory(tmp_dir.keep());

        let mut repo = factory
            .initialize_user_repository(&user_id, passwd.clone())
            .await
            .unwrap();
        repo.save().await.unwrap();

        assert!(factory.user_has_repository(&user_id));

        let result = factory.get_user_repository(&user_id, passwd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repo_open_with_wrong_password() {
        let tmp_dir = TempDir::new().unwrap();

        let user_id = "user_id".to_string();
        let passwd = "123";

        let factory = FileRepositoriesFactory(tmp_dir.keep());

        factory
            .initialize_user_repository(&user_id, passwd.to_string())
            .await
            .unwrap()
            .save()
            .await
            .unwrap();
        let result = factory
            .get_user_repository(&user_id, "312".to_string())
            .await
            .unwrap_err();

        assert!(matches!(result, RepositoryOpenError::WrongPassword));
    }
}
