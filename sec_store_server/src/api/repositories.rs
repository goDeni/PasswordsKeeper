use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use sec_store::repository::remote::{
    CreateRepositoryRequest, OpenRepositoryRequest, OpenRepositoryResponse,
};
use sec_store::repository::RepositoriesSource;

use super::SimpleStatus;
use crate::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/repositories/{repository_name}", post(create_repository))
        .route(
            "/repositories/{repository_name}/sessions",
            post(open_repository),
        )
}

async fn create_repository(
    State(state): State<AppState>,
    AxumPath(repository_name): AxumPath<String>,
    Json(request): Json<CreateRepositoryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .repositories
        .create_repository(&repository_name, request.password)
        .await
        .map_err(ApiError::from_create_error)?;

    Ok((StatusCode::CREATED, Json(SimpleStatus::new("created"))))
}

async fn open_repository(
    State(state): State<AppState>,
    AxumPath(repository_name): AxumPath<String>,
    Json(request): Json<OpenRepositoryRequest>,
) -> Result<Json<OpenRepositoryResponse>, ApiError> {
    let repository = state
        .repositories
        .open_repository(&repository_name, request.password)
        .await
        .map_err(ApiError::from_open_error)?;
    let session_id = state
        .insert_session(repository)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(OpenRepositoryResponse { session_id }))
}
