use std::path::Path;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use super::{
    AddRecordError, AddResult, CreateRepositoryError, CreateRepositoryResult, OpenResult,
    RecordsRepository, RepositoriesSource, RepositoryOpenError, UpdateRecordError, UpdateResult,
};
use crate::record::{Record, RecordId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepositoryRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRepositoryRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRepositoryResponse {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRecordRequest {
    pub record: Record,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecordRequest {
    pub record: Record,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct RemoteClientConfig {
    pub base_url: String,
    pub client_identity_pem_path: String,
    pub ca_cert_pem_path: String,
}

#[derive(Debug, Clone)]
pub struct RemoteRepositoriesClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct RemoteRecordsRepository {
    client: Client,
    base_url: String,
    pub session_id: String,
}

impl RemoteRepositoriesClient {
    pub async fn from_config(config: RemoteClientConfig) -> Result<Self> {
        let identity_pem = tokio::fs::read(&config.client_identity_pem_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to read client identity pem {}",
                    config.client_identity_pem_path
                )
            })?;
        let ca_cert_pem = tokio::fs::read(&config.ca_cert_pem_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to read CA certificate pem {}",
                    config.ca_cert_pem_path
                )
            })?;

        let client = Client::builder()
            .identity(
                reqwest::Identity::from_pem(&identity_pem)
                    .context("Failed to parse client identity pem")?,
            )
            .add_root_certificate(
                reqwest::Certificate::from_pem(&ca_cert_pem)
                    .context("Failed to parse CA certificate pem")?,
            )
            .use_rustls_tls()
            .https_only(true)
            .build()
            .context("Failed to create HTTPS client")?;

        Ok(Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn from_pem_files<P: AsRef<Path>, C: AsRef<Path>>(
        base_url: impl Into<String>,
        client_identity_pem_path: P,
        ca_cert_pem_path: C,
    ) -> Result<Self> {
        Self::from_config(RemoteClientConfig {
            base_url: base_url.into(),
            client_identity_pem_path: client_identity_pem_path
                .as_ref()
                .to_string_lossy()
                .into_owned(),
            ca_cert_pem_path: ca_cert_pem_path.as_ref().to_string_lossy().into_owned(),
        })
        .await
    }

    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let response = self
            .client
            .delete(self.url(&format!("/sessions/{session_id}")))
            .send()
            .await
            .context("Failed to close remote repository session")?;

        if response.status().is_success() || response.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        Err(anyhow!(
            "Failed to close remote repository session: {}",
            read_error(response).await
        ))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl RemoteRecordsRepository {
    pub async fn close(self) -> Result<()> {
        let client = RemoteRepositoriesClient {
            client: self.client,
            base_url: self.base_url,
        };
        client.close_session(&self.session_id).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[async_trait]
impl RepositoriesSource<RemoteRecordsRepository> for RemoteRepositoriesClient {
    async fn create_repository(
        &self,
        repository_name: &str,
        passwd: String,
    ) -> CreateRepositoryResult<RemoteRecordsRepository> {
        let open_password = passwd.clone();
        let response = self
            .client
            .post(self.url(&format!("/repositories/{repository_name}")))
            .json(&CreateRepositoryRequest { password: passwd })
            .send()
            .await
            .map_err(|err| CreateRepositoryError::UnexpectedError(err.into()))?;

        match response.status() {
            StatusCode::CREATED => self
                .open_repository(repository_name, open_password)
                .await
                .map_err(|err| CreateRepositoryError::UnexpectedError(err.into())),
            StatusCode::CONFLICT => Err(CreateRepositoryError::RepositoryAlreadyExists),
            _ => Err(CreateRepositoryError::UnexpectedError(anyhow!(
                read_error(response).await
            ))),
        }
    }

    async fn open_repository(
        &self,
        repository_name: &str,
        passwd: String,
    ) -> OpenResult<RemoteRecordsRepository> {
        let response = self
            .client
            .post(self.url(&format!("/repositories/{repository_name}/sessions")))
            .json(&OpenRepositoryRequest { password: passwd })
            .send()
            .await
            .map_err(|err| RepositoryOpenError::OpenError(err.into()))?;

        match response.status() {
            StatusCode::OK => {
                let response = response
                    .json::<OpenRepositoryResponse>()
                    .await
                    .map_err(|err| RepositoryOpenError::OpenError(err.into()))?;
                Ok(RemoteRecordsRepository {
                    client: self.client.clone(),
                    base_url: self.base_url.clone(),
                    session_id: response.session_id,
                })
            }
            StatusCode::UNAUTHORIZED => Err(RepositoryOpenError::WrongPassword),
            StatusCode::NOT_FOUND => Err(RepositoryOpenError::DoesntExist),
            _ => Err(RepositoryOpenError::OpenError(anyhow!(
                read_error(response).await
            ))),
        }
    }
}

#[async_trait]
impl RecordsRepository for RemoteRecordsRepository {
    async fn cancel(&mut self) -> Result<()> {
        simple_post(
            &self.client,
            &self.url(&format!("/sessions/{}/cancel", self.session_id)),
        )
        .await
    }

    async fn save(&mut self) -> Result<()> {
        simple_post(
            &self.client,
            &self.url(&format!("/sessions/{}/save", self.session_id)),
        )
        .await
    }

    async fn get_records(&self) -> Result<Vec<Record>> {
        let response = self
            .client
            .get(self.url(&format!("/sessions/{}/records", self.session_id)))
            .send()
            .await
            .context("Failed to get remote records")?;
        expect_json(response).await
    }

    async fn get(&self, record_id: &RecordId) -> Result<Option<Record>> {
        let response = self
            .client
            .get(self.url(&format!(
                "/sessions/{}/records/{record_id}",
                self.session_id
            )))
            .send()
            .await
            .context("Failed to get remote record")?;

        match response.status() {
            StatusCode::OK => response
                .json::<Record>()
                .await
                .map(Some)
                .context("Failed to parse remote record"),
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(anyhow!(read_error(response).await)),
        }
    }

    async fn update(&mut self, record: Record) -> UpdateResult<()> {
        let response = self
            .client
            .put(self.url(&format!(
                "/sessions/{}/records/{}",
                self.session_id, record.id
            )))
            .json(&UpdateRecordRequest { record })
            .send()
            .await
            .map_err(|err| UpdateRecordError::UnxpectedError(err.into()))?;
        expect_update_result(response).await
    }

    async fn delete(&mut self, record_id: &RecordId) -> UpdateResult<()> {
        let response = self
            .client
            .delete(self.url(&format!(
                "/sessions/{}/records/{record_id}",
                self.session_id
            )))
            .send()
            .await
            .map_err(|err| UpdateRecordError::UnxpectedError(err.into()))?;
        expect_update_result(response).await
    }

    async fn add_record(&mut self, record: Record) -> AddResult<()> {
        let response = self
            .client
            .post(self.url(&format!("/sessions/{}/records", self.session_id)))
            .json(&AddRecordRequest { record })
            .send()
            .await
            .map_err(|err| AddRecordError::UnxpectedError(err.into()))?;
        match response.status() {
            StatusCode::CREATED => Ok(()),
            StatusCode::CONFLICT => Err(AddRecordError::RecordDoesntExist),
            _ => Err(AddRecordError::UnxpectedError(anyhow!(
                read_error(response).await
            ))),
        }
    }

    async fn dump(&self) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(self.url(&format!("/sessions/{}/export", self.session_id)))
            .send()
            .await
            .context("Failed to export remote repository")?;

        if !response.status().is_success() {
            return Err(anyhow!(read_error(response).await));
        }

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .context("Failed to read remote repository export")
    }
}

async fn simple_post(client: &Client, url: &str) -> Result<()> {
    let response = client
        .post(url)
        .send()
        .await
        .with_context(|| format!("Failed POST {url}"))?;

    if response.status().is_success() {
        return Ok(());
    }

    Err(anyhow!(read_error(response).await))
}

async fn expect_json<T>(response: reqwest::Response) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    if !response.status().is_success() {
        return Err(anyhow!(read_error(response).await));
    }

    response
        .json::<T>()
        .await
        .context("Failed to decode JSON response")
}

async fn expect_update_result(response: reqwest::Response) -> UpdateResult<()> {
    match response.status() {
        StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
        StatusCode::NOT_FOUND => Err(UpdateRecordError::RecordDoesntExist),
        _ => Err(UpdateRecordError::UnxpectedError(anyhow!(
            read_error(response).await
        ))),
    }
}

async fn read_error(response: reqwest::Response) -> String {
    let status = response.status();
    match response.json::<ErrorResponse>().await {
        Ok(err) => format!("{status}: {}", err.error),
        Err(_) => format!("{status}"),
    }
}
